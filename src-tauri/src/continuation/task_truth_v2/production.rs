use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::super::{stable_hash, ContinueEvidencePreview, ContinueReturnTarget, ContinueWorkTruth};
use super::checkpoint;
use super::model::TASK_TRUTH_MODEL_OUTPUT_SCHEMA_V1;
use super::observation_packet::ObservationPacketV2;
use super::selection::SNAPSHOT_SELECTION_POLICY_V1;
use super::task_snapshot::{TaskSnapshotV2, TASK_SNAPSHOT_SCHEMA_V2};
use super::verifier::TASK_TRUTH_VERIFIER_VERSION;

pub(crate) const TASK_TRUTH_PUBLIC_ANSWER_SCHEMA_V1: &str = "smalltalk.task_truth_public_answer.v6";
pub(crate) const TASK_TRUTH_AUTHORITY_POLICY_V1: &str =
    "smalltalk.model_first_task_truth_authority_policy.v1";
pub(crate) const COMPACT_ADMISSION_VERSION: &str = "smalltalk.compact_semantic_admission.v1";

fn default_public_task_basis() -> String {
    "unresolved".into()
}

fn default_public_task_state() -> String {
    "unclear".into()
}

fn default_public_model_status() -> String {
    "unresolved".into()
}

fn default_target_status() -> String {
    "no_task".into()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskTruthAuthorityStateV1 {
    Off,
    Shadow,
    Eligible,
    Authoritative,
    Rollback,
}

impl TaskTruthAuthorityStateV1 {
    fn from_environment() -> Self {
        match std::env::var("SMALLTALK_TASK_TRUTH_AUTHORITY")
            .unwrap_or_else(|_| "shadow".into())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "off" => Self::Off,
            "eligible" => Self::Eligible,
            "authoritative" => Self::Authoritative,
            "rollback" => Self::Rollback,
            _ => Self::Shadow,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Shadow => "shadow",
            Self::Eligible => "eligible",
            Self::Authoritative => "authoritative",
            Self::Rollback => "rollback",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskTruthFieldSupportV1 {
    pub confidence: Option<f64>,
    pub support_status: String,
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub missing_evidence: Vec<String>,
    #[serde(default)]
    pub verifier_result: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskTruthFieldAdmissionV1 {
    pub verdict: String,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TaskTruthAlternativeV1 {
    pub hypothesis_id: String,
    pub task_summary: String,
    pub relation: String,
    pub confidence: f64,
    pub evidence_refs: Vec<String>,
    pub contradicting_evidence_refs: Vec<String>,
    pub task_thread_id: Option<String>,
    pub task_thread_revision: Option<i64>,
    pub last_supported_at_ms: Option<i64>,
    pub disposition: String,
    pub reason_codes: Vec<String>,
    pub semantic_payload: Option<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TaskTruthCurrentActivityV1 {
    pub observed_surface: Option<String>,
    pub immediate_user_operation: Option<String>,
    pub semantic_effect_of_operation: Option<String>,
    pub current_subtask: Option<String>,
    pub relationship_to_primary: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskTruthAtomicIdentityV1 {
    #[serde(default)]
    pub decision_id: String,
    #[serde(default)]
    pub current_frame_id: String,
    #[serde(default)]
    pub packet_policy_version: String,
    #[serde(default)]
    pub response_schema_version: String,
    #[serde(default)]
    pub admission_version: String,
    #[serde(default)]
    pub admitted_result_id: String,
    #[serde(default)]
    pub correction_watermark: String,
    #[serde(default)]
    pub target_identity: Option<String>,
    pub session_id: Option<String>,
    pub task_thread_id: Option<String>,
    pub task_thread_revision: Option<i64>,
    pub task_snapshot_id: String,
    pub snapshot_revision: i64,
    pub selected_hypothesis_id: Option<String>,
    pub model_request_id: Option<String>,
    pub model_response_id: Option<String>,
    pub observation_packet_id: String,
    pub evidence_watermark: String,
    pub correction_fingerprint: String,
}

pub(crate) const CONTINUE_PRODUCT_PROJECTION_SCHEMA_V1: &str =
    "smalltalk.continue_product_projection.v1";
const PRODUCT_INSTRUCTION_MAX_CHARS: usize = 160;
const PRODUCT_RESUME_CONTEXT_MAX_CHARS: usize = 180;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContinuePresentationStateV1 {
    ActionKnown,
    TaskKnownActionUnknown,
    TaskUnknown,
    ProviderFailure,
    ParserFailure,
    ValidationFailure,
    CaptureFailure,
    StaleDecision,
}

impl Default for ContinuePresentationStateV1 {
    fn default() -> Self {
        Self::TaskUnknown
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContinueProductActionKindV1 {
    OpenDirectTarget,
    InspectEvidence,
    RefreshContinue,
    None,
}

impl Default for ContinueProductActionKindV1 {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ContinueProductActionV1 {
    pub kind: ContinueProductActionKindV1,
    pub label: String,
}

/// The one backend-owned meaning rendered by both React and the native island.
/// Old stored answers deserialize safely through this type's defaults.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ContinueProductProjectionV1 {
    pub schema: String,
    pub answer_identity: String,
    pub presentation_state: ContinuePresentationStateV1,
    pub primary_instruction: String,
    pub resume_context: Option<String>,
    pub location_context: Option<String>,
    pub semantic_status: String,
    pub task_state: String,
    pub target_status: String,
    pub primary_action: ContinueProductActionV1,
    pub inspect_available: bool,
    pub unresolved_reason: Option<String>,
}

impl Default for ContinueProductProjectionV1 {
    fn default() -> Self {
        Self {
            schema: CONTINUE_PRODUCT_PROJECTION_SCHEMA_V1.into(),
            answer_identity: String::new(),
            presentation_state: ContinuePresentationStateV1::TaskUnknown,
            primary_instruction: "I couldn’t identify the unfinished task.".into(),
            resume_context: None,
            location_context: None,
            semantic_status: "unresolved".into(),
            task_state: "unclear".into(),
            target_status: "no_task".into(),
            primary_action: ContinueProductActionV1::default(),
            inspect_available: false,
            unresolved_reason: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TaskTruthRecentContextV1 {
    pub sequence_index: usize,
    pub app_label: String,
    pub site_hostname: Option<String>,
    pub first_observed_at_ms: i64,
    pub last_observed_at_ms: i64,
    pub is_current: bool,
    pub revisited: bool,
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub semantic_role: Option<String>,
    #[serde(default)]
    pub role_confidence: Option<f64>,
    #[serde(default)]
    pub relationship_to_primary_task: Option<String>,
    #[serde(default)]
    pub role_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskTruthPublicAnswerV1 {
    pub schema: String,
    #[serde(default = "default_public_task_basis")]
    pub task_basis: String,
    pub task_resolution_status: String,
    /// Raw compact-provider status. Public compatibility status may be locally
    /// downgraded after verification, but this value is never upgraded.
    #[serde(default = "default_public_model_status")]
    pub model_resolution_status: String,
    /// Exact provider semantic status. This is never locally rewritten.
    #[serde(default = "default_public_model_status")]
    pub raw_model_status: String,
    /// Field-admitted status. Admission may preserve or downgrade raw status,
    /// but can never increase certainty.
    #[serde(default = "default_public_model_status")]
    pub admitted_semantic_status: String,
    #[serde(default = "default_public_task_basis")]
    pub semantic_source_kind: String,
    #[serde(default)]
    pub field_admission: BTreeMap<String, TaskTruthFieldAdmissionV1>,
    #[serde(default)]
    pub claim_confidence: BTreeMap<String, f64>,
    #[serde(default = "default_target_status")]
    pub target_status: String,
    #[serde(default)]
    pub unresolved_or_failure_reason: Option<String>,
    #[serde(default)]
    pub semantic_conflicts: Vec<String>,
    #[serde(default)]
    pub atomic_answer_identity: String,
    #[serde(default)]
    pub product_projection: ContinueProductProjectionV1,
    #[serde(default = "default_stale_product_projection")]
    pub stale_product_projection: ContinueProductProjectionV1,
    /// LCA-02 authoritative meaning: the newest concrete unfinished objective.
    #[serde(default)]
    pub unfinished_task: Option<String>,
    /// LCA-02 authoritative lifecycle state. Compatibility execution fields are
    /// derived from this value for compact answers and never override it.
    #[serde(default = "default_public_task_state")]
    pub task_state: String,
    /// LCA-02 authoritative meaning: the exact meaningful state left behind.
    #[serde(default)]
    pub resume_point: Option<String>,
    /// LCA-02 authoritative meaning: one admitted, evidence-supported action.
    #[serde(default)]
    pub next_supported_action: Option<String>,
    /// Immediately relevant work already complete, never a second headline.
    #[serde(default)]
    pub completed_context: Option<String>,
    pub observed_surface: Option<String>,
    pub immediate_user_operation: Option<String>,
    pub semantic_effect_of_operation: Option<String>,
    pub current_subtask: Option<String>,
    pub current_activity: TaskTruthCurrentActivityV1,
    pub task_summary: Option<String>,
    pub task_object: Option<String>,
    pub last_meaningful_progress: Option<String>,
    pub unfinished_state: Option<String>,
    pub execution_state: String,
    pub next_action: Option<String>,
    pub where_summary: Option<String>,
    pub relationship_to_prior: String,
    #[serde(default)]
    pub recent_context: Vec<TaskTruthRecentContextV1>,
    pub alternative_hypotheses: Vec<TaskTruthAlternativeV1>,
    pub direct_return_target: Option<ContinueReturnTarget>,
    pub evidence_preview: Option<ContinueEvidencePreview>,
    pub field_support: BTreeMap<String, TaskTruthFieldSupportV1>,
    pub task_understanding_source: String,
    pub wording_source: String,
    pub target_selection_source: String,
    pub snapshot_id: String,
    pub snapshot_revision: i64,
    pub evidence_watermark: String,
    pub semantic_source: String,
    pub provider_name: Option<String>,
    pub provider_model: Option<String>,
    pub request_id: Option<String>,
    pub response_id: Option<String>,
    pub selected_hypothesis_id: Option<String>,
    pub inference_status: String,
    pub atomic_identity: TaskTruthAtomicIdentityV1,
}

impl Default for TaskTruthPublicAnswerV1 {
    fn default() -> Self {
        Self {
            schema: TASK_TRUTH_PUBLIC_ANSWER_SCHEMA_V1.into(),
            task_basis: "unresolved".into(),
            task_resolution_status: "unresolved".into(),
            model_resolution_status: "unresolved".into(),
            raw_model_status: "unresolved".into(),
            admitted_semantic_status: "unresolved".into(),
            semantic_source_kind: "unresolved".into(),
            field_admission: BTreeMap::new(),
            claim_confidence: BTreeMap::new(),
            target_status: "no_task".into(),
            unresolved_or_failure_reason: None,
            semantic_conflicts: Vec::new(),
            atomic_answer_identity: String::new(),
            product_projection: ContinueProductProjectionV1::default(),
            stale_product_projection: default_stale_product_projection(),
            unfinished_task: None,
            task_state: "unclear".into(),
            resume_point: None,
            next_supported_action: None,
            completed_context: None,
            observed_surface: None,
            immediate_user_operation: None,
            semantic_effect_of_operation: None,
            current_subtask: None,
            current_activity: TaskTruthCurrentActivityV1 {
                relationship_to_primary: "unrelated_or_unknown".into(),
                ..Default::default()
            },
            task_summary: None,
            task_object: None,
            last_meaningful_progress: None,
            unfinished_state: None,
            execution_state: "unclear".into(),
            next_action: None,
            where_summary: None,
            relationship_to_prior: "unrelated_or_unknown".into(),
            recent_context: Vec::new(),
            alternative_hypotheses: Vec::new(),
            direct_return_target: None,
            evidence_preview: None,
            field_support: BTreeMap::new(),
            task_understanding_source: "unresolved".into(),
            wording_source: "deterministic".into(),
            target_selection_source: "strict_local_policy".into(),
            snapshot_id: String::new(),
            snapshot_revision: 0,
            evidence_watermark: String::new(),
            semantic_source: "unresolved".into(),
            provider_name: None,
            provider_model: None,
            request_id: None,
            response_id: None,
            selected_hypothesis_id: None,
            inference_status: "no_inference".into(),
            atomic_identity: TaskTruthAtomicIdentityV1::default(),
        }
    }
}

fn bounded_product_text(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    // Character counts, rather than bytes, keep this deterministic for Unicode.
    // The ellipsis is included inside the documented limit.
    value
        .chars()
        .take(max_chars.saturating_sub(1))
        .chain(std::iter::once('…'))
        .collect()
}

fn nonempty_product_text(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn product_failure_state(answer: &TaskTruthPublicAnswerV1) -> Option<ContinuePresentationStateV1> {
    let reason = format!(
        "{} {}",
        answer
            .unresolved_or_failure_reason
            .as_deref()
            .unwrap_or_default(),
        answer.inference_status
    )
    .to_ascii_lowercase();
    if ["capture", "screenshot", "manual_continue_boundary"]
        .iter()
        .any(|marker| reason.contains(marker))
    {
        Some(ContinuePresentationStateV1::CaptureFailure)
    } else if [
        "parser",
        "parse",
        "invalid_json",
        "invalid_response",
        "schema",
    ]
    .iter()
    .any(|marker| reason.contains(marker))
    {
        Some(ContinuePresentationStateV1::ParserFailure)
    } else if [
        "provider",
        "transport",
        "timeout",
        "http_",
        "request_failed",
        "refused",
    ]
    .iter()
    .any(|marker| reason.contains(marker))
    {
        Some(ContinuePresentationStateV1::ProviderFailure)
    } else if [
        "validation",
        "verification",
        "verifier",
        "support_",
        "invalid_atomic_identity",
        "rejected",
    ]
    .iter()
    .any(|marker| reason.contains(marker))
    {
        Some(ContinuePresentationStateV1::ValidationFailure)
    } else {
        None
    }
}

pub(crate) fn continue_product_projection(
    answer: &TaskTruthPublicAnswerV1,
) -> ContinueProductProjectionV1 {
    let action = nonempty_product_text(answer.next_supported_action.as_deref());
    let task = nonempty_product_text(answer.unfinished_task.as_deref());
    let answer_identity = nonempty_product_text(Some(&answer.atomic_answer_identity))
        .or_else(|| nonempty_product_text(Some(&answer.atomic_identity.admitted_result_id)))
        .unwrap_or_default()
        .to_string();
    let inspect_available = answer.evidence_preview.is_some()
        || !answer_identity.is_empty()
        || !answer.field_admission.is_empty()
        || !answer.field_support.is_empty()
        || !answer.recent_context.is_empty();

    let presentation_state = if answer.target_status == "stale_decision" {
        ContinuePresentationStateV1::StaleDecision
    } else if let Some(failure_state) = product_failure_state(answer) {
        failure_state
    } else if action.is_some() {
        ContinuePresentationStateV1::ActionKnown
    } else if task.is_some() {
        ContinuePresentationStateV1::TaskKnownActionUnknown
    } else {
        ContinuePresentationStateV1::TaskUnknown
    };

    let primary_instruction = match presentation_state {
        ContinuePresentationStateV1::ActionKnown => {
            // This is intentionally the only semantic source for an actionable
            // headline. `unfinished_state` and all compatibility fields are ignored.
            bounded_product_text(action.unwrap_or_default(), PRODUCT_INSTRUCTION_MAX_CHARS)
        }
        ContinuePresentationStateV1::TaskKnownActionUnknown => {
            "I found the task, but not a safe next step.".into()
        }
        ContinuePresentationStateV1::TaskUnknown => {
            "I couldn’t identify the unfinished task.".into()
        }
        ContinuePresentationStateV1::ProviderFailure => {
            "The provider couldn’t produce a usable Continue answer.".into()
        }
        ContinuePresentationStateV1::ParserFailure => "I couldn’t read the Continue answer.".into(),
        ContinuePresentationStateV1::ValidationFailure => {
            "The Continue answer did not pass validation.".into()
        }
        ContinuePresentationStateV1::CaptureFailure => {
            "I couldn’t capture the current task boundary.".into()
        }
        ContinuePresentationStateV1::StaleDecision => {
            "The saved answer is older than the latest work.".into()
        }
    };

    let resume_point = nonempty_product_text(answer.resume_point.as_deref());
    let completed_context = nonempty_product_text(answer.completed_context.as_deref());
    let resume_context = match (resume_point, completed_context) {
        (Some(resume), Some(completed))
            if answer.task_state == "needs_user_verification"
                && !resume
                    .to_ascii_lowercase()
                    .contains(&completed.to_ascii_lowercase()) =>
        {
            Some(bounded_product_text(
                &format!("{}; {}", completed.trim_end_matches('.'), resume),
                PRODUCT_RESUME_CONTEXT_MAX_CHARS,
            ))
        }
        (Some(resume), _) => Some(bounded_product_text(
            resume,
            PRODUCT_RESUME_CONTEXT_MAX_CHARS,
        )),
        (None, Some(completed)) => Some(bounded_product_text(
            completed,
            PRODUCT_RESUME_CONTEXT_MAX_CHARS,
        )),
        (None, None) => {
            task.map(|task| bounded_product_text(task, PRODUCT_RESUME_CONTEXT_MAX_CHARS))
        }
    };

    let primary_action = match presentation_state {
        ContinuePresentationStateV1::StaleDecision => ContinueProductActionV1 {
            kind: ContinueProductActionKindV1::RefreshContinue,
            label: "Refresh Continue".into(),
        },
        ContinuePresentationStateV1::ActionKnown
            if answer.target_status == "direct_target_ready"
                && answer.direct_return_target.is_some() =>
        {
            ContinueProductActionV1 {
                kind: ContinueProductActionKindV1::OpenDirectTarget,
                label: "Continue here".into(),
            }
        }
        ContinuePresentationStateV1::ActionKnown if answer.evidence_preview.is_some() => {
            ContinueProductActionV1 {
                kind: ContinueProductActionKindV1::InspectEvidence,
                label: "View last screen".into(),
            }
        }
        ContinuePresentationStateV1::ProviderFailure
        | ContinuePresentationStateV1::ParserFailure
        | ContinuePresentationStateV1::CaptureFailure
        | ContinuePresentationStateV1::TaskUnknown => ContinueProductActionV1 {
            kind: ContinueProductActionKindV1::RefreshContinue,
            label: "Try Continue again".into(),
        },
        ContinuePresentationStateV1::ValidationFailure if inspect_available => {
            ContinueProductActionV1 {
                kind: ContinueProductActionKindV1::InspectEvidence,
                label: "Inspect".into(),
            }
        }
        _ if inspect_available => ContinueProductActionV1 {
            kind: ContinueProductActionKindV1::InspectEvidence,
            label: "Inspect".into(),
        },
        _ => ContinueProductActionV1::default(),
    };

    ContinueProductProjectionV1 {
        schema: CONTINUE_PRODUCT_PROJECTION_SCHEMA_V1.into(),
        answer_identity,
        presentation_state,
        primary_instruction,
        resume_context,
        location_context: nonempty_product_text(answer.where_summary.as_deref())
            .map(|value| bounded_product_text(value, PRODUCT_RESUME_CONTEXT_MAX_CHARS)),
        semantic_status: answer.admitted_semantic_status.clone(),
        task_state: answer.task_state.clone(),
        target_status: answer.target_status.clone(),
        primary_action,
        inspect_available,
        unresolved_reason: answer.unresolved_or_failure_reason.clone(),
    }
}

fn default_stale_product_projection() -> ContinueProductProjectionV1 {
    stale_continue_product_projection(&ContinueProductProjectionV1::default())
}

fn stale_product_projection_for_current(
    current: &ContinueProductProjectionV1,
) -> ContinueProductProjectionV1 {
    if matches!(
        current.presentation_state,
        ContinuePresentationStateV1::ActionKnown
            | ContinuePresentationStateV1::TaskKnownActionUnknown
            | ContinuePresentationStateV1::StaleDecision
    ) {
        stale_continue_product_projection(current)
    } else {
        // A changed evidence watermark invalidates reuse and direct opening,
        // but it does not turn a typed acquisition/provider/parser/validation/
        // capture failure into an older semantic answer.
        current.clone()
    }
}

pub(crate) fn stale_continue_product_projection(
    current: &ContinueProductProjectionV1,
) -> ContinueProductProjectionV1 {
    ContinueProductProjectionV1 {
        schema: CONTINUE_PRODUCT_PROJECTION_SCHEMA_V1.into(),
        answer_identity: current.answer_identity.clone(),
        presentation_state: ContinuePresentationStateV1::StaleDecision,
        primary_instruction: "The saved answer is older than the latest work.".into(),
        resume_context: current.resume_context.clone(),
        location_context: None,
        semantic_status: current.semantic_status.clone(),
        task_state: current.task_state.clone(),
        target_status: "stale_decision".into(),
        primary_action: ContinueProductActionV1 {
            kind: ContinueProductActionKindV1::RefreshContinue,
            label: "Refresh Continue".into(),
        },
        inspect_available: true,
        unresolved_reason: Some("material_evidence_watermark_advanced".into()),
    }
}

impl TaskTruthPublicAnswerV1 {
    pub(crate) fn recompute_product_projection(&mut self) {
        let product_projection = continue_product_projection(self);
        self.stale_product_projection = stale_product_projection_for_current(&product_projection);
        self.product_projection = product_projection;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskTruthInferenceDiagnosticV1 {
    pub schema: String,
    pub status: String,
    pub origin: String,
    pub provider: String,
    pub model: String,
    pub request_id: Option<String>,
    pub provider_request_id: Option<String>,
    pub response_id: Option<String>,
    pub provider_attempt_count: usize,
    pub latency_ms: i64,
    pub image_count: usize,
    pub image_bytes: usize,
    pub estimated_tokens: usize,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub estimated_cost_usd: Option<f64>,
    pub verification_status: String,
    pub selected_hypothesis_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskTruthProductionDecisionV1 {
    pub schema: String,
    pub requested_state: TaskTruthAuthorityStateV1,
    pub effective_state: TaskTruthAuthorityStateV1,
    pub policy_version: String,
    pub release_gate_passed: bool,
    pub release_gate_source: Option<String>,
    pub reason_codes: Vec<String>,
    pub cache_fingerprint: String,
    pub answer: Option<TaskTruthPublicAnswerV1>,
    pub inference_diagnostic: Option<TaskTruthInferenceDiagnosticV1>,
}

impl Default for TaskTruthProductionDecisionV1 {
    fn default() -> Self {
        Self {
            schema: "smalltalk.task_truth_production_decision.v1".into(),
            requested_state: TaskTruthAuthorityStateV1::Shadow,
            effective_state: TaskTruthAuthorityStateV1::Shadow,
            policy_version: TASK_TRUTH_AUTHORITY_POLICY_V1.into(),
            release_gate_passed: false,
            release_gate_source: None,
            reason_codes: vec!["release_gate_not_evaluated".into()],
            cache_fingerprint: String::new(),
            answer: None,
            inference_diagnostic: None,
        }
    }
}

fn inference_diagnostic(
    conn: &Connection,
    packet_id: Option<&str>,
    decision_id: Option<&str>,
) -> Result<Option<TaskTruthInferenceDiagnosticV1>, String> {
    let raw = if let Some(decision_id) = decision_id {
        conn.query_row(
            "SELECT packet_summary_json
             FROM task_truth_v2_shadow_audits
             WHERE decision_id = ?1
             ORDER BY observed_at_ms DESC LIMIT 1",
            params![decision_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
    } else if let Some(packet_id) = packet_id {
        conn.query_row(
            "SELECT packet_summary_json
             FROM task_truth_v2_shadow_audits
             WHERE packet_id = ?1
             ORDER BY observed_at_ms DESC LIMIT 1",
            params![packet_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
    } else {
        None
    };
    let Some(raw) = raw else {
        return Ok(None);
    };
    let value: Value = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    let Some(multimodal) = value.get("multimodal") else {
        return Ok(None);
    };
    let resolver = multimodal.get("resolver").unwrap_or(&Value::Null);
    let request = resolver.get("request_audit").unwrap_or(&Value::Null);
    let usage = resolver.get("usage").unwrap_or(&Value::Null);
    Ok(Some(TaskTruthInferenceDiagnosticV1 {
        schema: "smalltalk.task_truth_inference_diagnostic.v1".into(),
        status: resolver
            .get("diagnostic_status")
            .and_then(Value::as_str)
            .unwrap_or("invalid_response")
            .into(),
        origin: resolver
            .get("origin")
            .and_then(Value::as_str)
            .unwrap_or("none")
            .into(),
        provider: resolver
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("unconfigured")
            .into(),
        model: resolver
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("unconfigured")
            .into(),
        request_id: resolver
            .get("request_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        provider_request_id: resolver
            .get("provider_request_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        response_id: resolver
            .get("response_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        provider_attempt_count: resolver
            .get("provider_attempts")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or_else(|| if request.is_null() { 0 } else { 1 }),
        latency_ms: resolver
            .get("latency_ms")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        image_count: request
            .get("image_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        image_bytes: request
            .get("image_bytes")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        estimated_tokens: request
            .get("estimated_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        input_tokens: usage.get("input_tokens").and_then(Value::as_i64),
        output_tokens: usage.get("output_tokens").and_then(Value::as_i64),
        total_tokens: usage.get("total_tokens").and_then(Value::as_i64),
        estimated_cost_usd: multimodal
            .get("estimated_request_cost_usd")
            .and_then(Value::as_f64),
        verification_status: multimodal
            .get("verification")
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .unwrap_or("verification_rejected")
            .into(),
        selected_hypothesis_id: multimodal
            .get("verification")
            .and_then(|value| value.get("snapshot"))
            .and_then(|value| value.get("selected_hypothesis_id"))
            .and_then(Value::as_str)
            .map(str::to_string),
    }))
}

fn release_report_path() -> PathBuf {
    std::env::var("SMALLTALK_TASK_TRUTH_RELEASE_REPORT")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/continue_accuracy/task_truth_v2/model_first/final-release-report.v1.json")
        })
}

fn release_report_identity(source: Option<&str>) -> Option<String> {
    source
        .and_then(|path| fs::read(path).ok())
        .map(|bytes| stable_hash(&bytes))
}

#[allow(dead_code)]
fn historical_tt2_release_report_is_complete(value: &serde_json::Value) -> bool {
    const METRICS: [&str; 13] = [
        "wrong_primary_task_rate",
        "control_navigation_as_task_rate",
        "useful_non_generic_task_summary",
        "task_object_accuracy",
        "execution_state_accuracy",
        "supported_next_action_precision",
        "supported_next_action_coverage",
        "return_target_precision",
        "unsupported_specific_claim_rate",
        "stronger_manual_result_downgraded",
        "unseen_application_useful_summary",
        "human_immediately_useful",
        "model_on_off_unexplained_task_disagreement",
    ];
    const SURFACES: [&str; 10] = [
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
    if value.get("schema").and_then(serde_json::Value::as_str)
        != Some("smalltalk.task_truth_v2.final_release_report.v1")
        || value
            .get("policy_version")
            .and_then(serde_json::Value::as_str)
            != Some("tt2.02-v1")
        || value.get("passed").and_then(serde_json::Value::as_bool) != Some(true)
        || value
            .get("authority_state")
            .and_then(serde_json::Value::as_str)
            != Some("authoritative")
        || value
            .get("violations")
            .and_then(serde_json::Value::as_array)
            .is_none_or(|items| !items.is_empty())
        || value
            .get("reviewed_live_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            < 200
        || value
            .get("locked_holdout_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            < 50
        || value
            .get("manual_scenario_count")
            .and_then(serde_json::Value::as_u64)
            != Some(14)
    {
        return false;
    }
    for field in [
        "evaluator_release_gate_passed",
        "evaluator_validation_passed",
        "manual_macos_qa_passed",
        "performance_cost_privacy_passed",
        "release_budget_policy_passed",
    ] {
        if value.get(field).and_then(serde_json::Value::as_bool) != Some(true) {
            return false;
        }
    }
    for metric in METRICS {
        let Some(assessment) = value
            .get("tt2_05_metric_results")
            .and_then(|metrics| metrics.get(metric))
        else {
            return false;
        };
        if assessment
            .get("passed")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
            || assessment
                .get("denominator")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                == 0
        {
            return false;
        }
        let Some(interval) = value
            .get("tt2_05_confidence_intervals")
            .and_then(|intervals| intervals.get(metric))
        else {
            return false;
        };
        if interval.get("method").and_then(serde_json::Value::as_str) != Some("wilson_score")
            || interval
                .get("lower")
                .and_then(serde_json::Value::as_f64)
                .is_none()
            || interval
                .get("upper")
                .and_then(serde_json::Value::as_f64)
                .is_none()
        {
            return false;
        }
    }
    for surface in SURFACES {
        let Some(assessment) = value
            .get("tt2_05_surface_wrong_task_results")
            .and_then(|surfaces| surfaces.get(surface))
        else {
            return false;
        };
        if assessment
            .get("passed")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
            || assessment
                .get("denominator")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                < 15
        {
            return false;
        }
        let interval_key = format!("wrong_primary_task_rate.surface.{surface}");
        let Some(interval) = value
            .get("tt2_05_confidence_intervals")
            .and_then(|intervals| intervals.get(&interval_key))
        else {
            return false;
        };
        if interval.get("method").and_then(serde_json::Value::as_str) != Some("wilson_score")
            || interval
                .get("lower")
                .and_then(serde_json::Value::as_f64)
                .is_none()
            || interval
                .get("upper")
                .and_then(serde_json::Value::as_f64)
                .is_none()
        {
            return false;
        }
    }
    for (slice, minimum) in [
        ("interruption_resumption", 30_u64),
        ("ambiguous_or_privacy_blocked", 20),
        ("waiting_on_agent_or_application", 20),
        ("completed_vs_new_task", 20),
    ] {
        if value
            .get("slice_denominators")
            .and_then(|slices| slices.get(slice))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            < minimum
        {
            return false;
        }
    }
    [
        "control_navigation_as_task",
        "stronger_manual_result_downgraded",
        "model_on_off_unexplained_task_disagreement",
        "privacy_violations",
        "unsafe_opens",
        "background_multimodal_requests",
    ]
    .iter()
    .all(|field| {
        value
            .get("zero_tolerance")
            .and_then(|counts| counts.get(*field))
            .and_then(serde_json::Value::as_u64)
            == Some(0)
    })
}

fn final_release_report_is_complete(value: &serde_json::Value) -> bool {
    const METRICS: [&str; 19] = [
        "wrong_primary_task_rate",
        "visible_surface_substituted_for_task",
        "wrong_activity_to_task_relationship",
        "wrong_task_switch_or_detour",
        "cross_session_stale_leakage",
        "mixed_snapshot_semantic_fields",
        "control_navigation_as_task_rate",
        "unsupported_specific_claim_rate",
        "provider_failure_local_semantic_fallback",
        "provider_failure_honest_unresolved",
        "useful_non_generic_task_summary",
        "task_object_accuracy",
        "execution_state_accuracy",
        "supported_next_action_precision",
        "supported_next_action_coverage",
        "return_target_precision",
        "stronger_manual_result_downgraded",
        "unseen_application_useful_summary",
        "human_immediately_useful",
    ];
    const SURFACES: [&str; 10] = [
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
    const BINDINGS: [&str; 14] = [
        "corpus_sha256",
        "holdout_sha256",
        "provider",
        "model",
        "prompt_version",
        "response_schema_version",
        "observation_packet_version",
        "verifier_version",
        "task_thread_version",
        "public_answer_version",
        "performance_privacy_policy_version",
        "manual_qa_manifest_sha256",
        "source_commit",
        "build_identity",
    ];
    if value.get("schema").and_then(Value::as_str)
        != Some("smalltalk.mfti_04.final_release_report.v1")
        || value.get("policy_version").and_then(Value::as_str) != Some("mfti.04-v1")
        || value.get("passed").and_then(Value::as_bool) != Some(true)
        || value.get("authority_state").and_then(Value::as_str) != Some("authoritative")
        || value
            .get("violations")
            .and_then(Value::as_array)
            .is_none_or(|v| !v.is_empty())
        || value
            .get("reviewed_live_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            < 200
        || value
            .get("locked_holdout_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            < 50
        || value.get("manual_scenario_count").and_then(Value::as_u64) != Some(10)
        || value
            .get("performance_sample_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            < 30
    {
        return false;
    }
    let validations_clear = [
        "policy",
        "evaluator",
        "release_identity",
        "manual_macos_qa",
        "performance_cost_privacy",
    ]
    .iter()
    .all(|field| {
        value
            .get("validation")
            .and_then(|v| v.get(*field))
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
    });
    if !validations_clear
        || !BINDINGS.iter().all(|field| {
            value
                .get("bindings")
                .and_then(|v| v.get(*field))
                .and_then(Value::as_str)
                .is_some_and(|v| !v.trim().is_empty())
        })
    {
        return false;
    }
    let metrics_pass = METRICS.iter().all(|metric| {
        let assessment = value.get("metric_results").and_then(|v| v.get(*metric));
        let interval = value
            .get("confidence_intervals")
            .and_then(|v| v.get(*metric));
        assessment
            .and_then(|v| v.get("passed"))
            .and_then(Value::as_bool)
            == Some(true)
            && assessment
                .and_then(|v| v.get("denominator"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0
            && interval
                .and_then(|v| v.get("method"))
                .and_then(Value::as_str)
                == Some("wilson_score")
            && interval
                .and_then(|v| v.get("lower"))
                .and_then(Value::as_f64)
                .is_some()
            && interval
                .and_then(|v| v.get("upper"))
                .and_then(Value::as_f64)
                .is_some()
    });
    metrics_pass
        && SURFACES.iter().all(|surface| {
            let assessment = value.get("surface_results").and_then(|v| v.get(*surface));
            assessment
                .and_then(|v| v.get("passed"))
                .and_then(Value::as_bool)
                == Some(true)
                && assessment
                    .and_then(|v| v.get("denominator"))
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    >= 15
        })
}

fn read_release_gate() -> (bool, Option<String>, Vec<String>) {
    let path = release_report_path();
    let source = Some(path.to_string_lossy().to_string());
    let Ok(bytes) = fs::read(&path) else {
        return (false, source, vec!["release_report_unreadable".into()]);
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return (false, source, vec!["release_report_invalid_json".into()]);
    };
    if final_release_report_is_complete(&value) {
        (true, source, vec!["locked_release_gate_passed".into()])
    } else {
        (false, source, vec!["release_gate_failed".into()])
    }
}

pub(crate) fn authoritative_runtime_enabled() -> bool {
    TaskTruthAuthorityStateV1::from_environment() == TaskTruthAuthorityStateV1::Authoritative
        && read_release_gate().0
}

fn clear_unsupported_semantics(answer: &mut TaskTruthPublicAnswerV1, reason: &str) {
    answer.task_resolution_status = "unresolved".into();
    answer.admitted_semantic_status = "unresolved".into();
    answer.unfinished_task = None;
    answer.task_state = "unclear".into();
    answer.resume_point = None;
    answer.next_supported_action = None;
    answer.completed_context = None;
    answer.observed_surface = None;
    answer.immediate_user_operation = None;
    answer.semantic_effect_of_operation = None;
    answer.current_subtask = None;
    answer.current_activity = TaskTruthCurrentActivityV1 {
        relationship_to_primary: "unrelated_or_unknown".into(),
        ..Default::default()
    };
    answer.task_summary = None;
    answer.task_object = None;
    answer.last_meaningful_progress = None;
    answer.unfinished_state = None;
    answer.execution_state = "unclear".into();
    answer.next_action = None;
    answer.where_summary = None;
    answer.relationship_to_prior = "unrelated_or_unknown".into();
    answer.alternative_hypotheses.clear();
    answer.direct_return_target = None;
    answer.target_status = "no_task".into();
    answer.unresolved_or_failure_reason = Some(reason.into());
    answer.field_support.clear();
    answer.task_understanding_source = "unresolved".into();
    answer.semantic_source = "unresolved".into();
    answer.selected_hypothesis_id = None;
    answer.inference_status = reason.into();
    answer.atomic_identity.selected_hypothesis_id = None;
    answer.recompute_product_projection();
}

fn enforce_model_first_semantic_authority(answer: &mut TaskTruthPublicAnswerV1) -> Option<String> {
    if answer.task_resolution_status == "unresolved" {
        answer.semantic_source = "unresolved".into();
        answer.task_understanding_source = "unresolved".into();
        return None;
    }

    let source_allowed = matches!(
        answer.semantic_source.as_str(),
        "cloud_multimodal_model" | "human_correction"
    );
    // A parsed provider response always receives a stable local envelope
    // identity. Some provider responses do not expose a provider-native id;
    // that must weaken provenance, not erase the model's text.
    let model_identity_present = answer.semantic_source != "cloud_multimodal_model"
        || answer
            .atomic_identity
            .model_response_id
            .as_deref()
            .is_some_and(|id| !id.trim().is_empty());
    let atomic_selection_present = answer
        .atomic_identity
        .task_thread_id
        .as_deref()
        .is_some_and(|id| !id.trim().is_empty())
        && answer.atomic_identity.task_thread_revision.is_some()
        && answer
            .atomic_identity
            .selected_hypothesis_id
            .as_deref()
            .is_some_and(|id| !id.trim().is_empty())
        && !answer.atomic_identity.task_snapshot_id.trim().is_empty()
        && !answer
            .atomic_identity
            .observation_packet_id
            .trim()
            .is_empty()
        && !answer.atomic_identity.evidence_watermark.trim().is_empty();

    if source_allowed && model_identity_present && atomic_selection_present {
        return None;
    }

    let reason = if !source_allowed {
        "unsupported_semantic_source"
    } else if !model_identity_present {
        "missing_model_response_identity"
    } else {
        "invalid_atomic_identity"
    };
    clear_unsupported_semantics(answer, reason);
    Some(reason.into())
}
#[derive(Debug)]
struct PftuProbePublicResult {
    answer: Option<TaskTruthPublicAnswerV1>,
    diagnostic: TaskTruthInferenceDiagnosticV1,
}

#[derive(Debug)]
struct PersistedCorrectionProjection {
    snapshot: TaskSnapshotV2,
    answer_id: String,
    correction_fingerprint: String,
    target_binding_id: Option<String>,
    current_frame_id: String,
    cutoff_observed_at_ms: i64,
}

/// A semantic probe can reject one unsupported field while still preserving
/// the other fields that passed local evidence admission. Only these two run
/// states are allowed to project the persisted admitted output. Transport,
/// parse, privacy, human-rejection, and maintenance-invalidation states remain
/// categorically ineligible even if a stale JSON value is present.
fn probe_status_allows_admitted_output(status: &str) -> bool {
    matches!(status, "success" | "support_slot_validation_failure")
}

/// Identity and diagnostic envelope for projecting one admitted compact result.
/// Keeping this separate from SQLite lets production and deterministic replay
/// exercise the exact same mapper without creating a second answer engine.
#[derive(Debug, Clone)]
pub(crate) struct CompactProbePublicMappingContext {
    pub(crate) decision_id: String,
    pub(crate) session_id: Option<String>,
    pub(crate) packet_id: String,
    pub(crate) evidence_watermark: String,
    pub(crate) configured_model: String,
    pub(crate) response_model: Option<String>,
    pub(crate) request_id: Option<String>,
    pub(crate) provider_request_id: Option<String>,
    pub(crate) provider_response_id: Option<String>,
    pub(crate) diagnostic_status: String,
    pub(crate) validation_issues: Vec<String>,
    pub(crate) recent_context: Vec<TaskTruthRecentContextV1>,
    pub(crate) current_frame_id: String,
    pub(crate) packet_policy_version: String,
    pub(crate) response_schema_version: String,
    pub(crate) explicit_goal_support_slots: Vec<String>,
    pub(crate) correction_watermark: String,
    pub(crate) semantic_conflicts: Vec<String>,
}

fn probe_status_label(status: super::semantic_probe::ProbeResolutionStatus) -> &'static str {
    match status {
        super::semantic_probe::ProbeResolutionStatus::Resolved => "resolved",
        super::semantic_probe::ProbeResolutionStatus::PartlyResolved => "partly_resolved",
        super::semantic_probe::ProbeResolutionStatus::Unresolved => "unresolved",
        super::semantic_probe::ProbeResolutionStatus::Refused => "refused",
    }
}

fn has_qualified_semantic_wording(output: &super::semantic_probe::ProbeModelOutput) -> bool {
    output
        .unfinished_task
        .iter()
        .chain(output.resume_point.iter())
        .chain(output.next_supported_action.iter())
        .any(|value| {
            let normalized = format!(" {} ", value.to_ascii_lowercase());
            [
                " likely ",
                " appears ",
                " appears to ",
                " may ",
                " might ",
                " seems ",
                " suspected ",
                " hypothesis ",
                " possibly ",
                " probably ",
            ]
            .iter()
            .any(|marker| normalized.contains(marker))
        })
}

fn compact_admitted_status(
    output: &super::semantic_probe::ProbeModelOutput,
    semantic_conflicts: &[String],
) -> &'static str {
    use super::semantic_probe::ProbeResolutionStatus as Raw;
    match output.status {
        Raw::Refused => return "refused",
        Raw::Unresolved => return "unresolved",
        _ => {}
    }
    if output.unfinished_task.is_none() || !semantic_conflicts.is_empty() {
        return "unresolved";
    }
    if output.status == Raw::PartlyResolved {
        return "partly_resolved";
    }
    let required_fields_present = output.task_state
        != super::semantic_probe::ProbeTaskState::Unclear
        && output.resume_point.is_some()
        && output.next_supported_action.is_some();
    let high_confidence = [
        "unfinished_task",
        "task_state",
        "resume_point",
        "next_supported_action",
    ]
    .into_iter()
    .all(|field| {
        output
            .confidence_by_field
            .get(field)
            .copied()
            .is_some_and(|score| {
                super::super::confidence::ConfidenceLabel::from_score(score)
                    == super::super::confidence::ConfidenceLabel::High
            })
    });
    if required_fields_present && high_confidence && !has_qualified_semantic_wording(output) {
        "resolved"
    } else {
        "partly_resolved"
    }
}

fn field_admission_verdict(reasons: &[String]) -> &'static str {
    if reasons.iter().any(|reason| reason.contains("private")) {
        "rejected_private"
    } else if reasons.iter().any(|reason| reason.contains("stale")) {
        "rejected_stale"
    } else if reasons.iter().any(|reason| {
        reason.contains("wrong_surface") || reason.contains("slot_category_not_allowed")
    }) {
        "rejected_wrong_surface"
    } else if reasons.iter().any(|reason| reason.contains("chronology")) {
        "rejected_chronology"
    } else if reasons.iter().any(|reason| reason.contains("contradict")) {
        "rejected_contradiction"
    } else if reasons.iter().any(|reason| reason.contains("generic")) {
        "rejected_generic"
    } else if reasons.iter().any(|reason| reason.contains("too_long")) {
        "rejected_overlong"
    } else if reasons.iter().any(|reason| {
        reason.contains("invalid")
            || reason.contains("inline_support_token")
            || reason.contains("refused_status")
    }) {
        "rejected_invalid_state"
    } else {
        "rejected_unsupported"
    }
}

fn compact_field_admission(
    output: &super::semantic_probe::ProbeModelOutput,
    validation_issues: &[String],
) -> BTreeMap<String, TaskTruthFieldAdmissionV1> {
    [
        "unfinished_task",
        "task_state",
        "resume_point",
        "next_supported_action",
        "completed_context",
        "where_summary",
    ]
    .into_iter()
    .map(|field| {
        let mut reasons = validation_issues
            .iter()
            .filter_map(|issue| issue.strip_prefix(&format!("{field}:")).map(str::to_string))
            .collect::<Vec<_>>();
        reasons.sort();
        reasons.dedup();
        let result = output
            .verifier_result_by_field
            .get(field)
            .copied()
            .unwrap_or_default();
        let verdict = match result {
            super::semantic_probe::ProbeFieldVerifierResult::Admitted => "accepted",
            super::semantic_probe::ProbeFieldVerifierResult::NotProposed => {
                reasons.push("not_proposed".into());
                "rejected_unsupported"
            }
            super::semantic_probe::ProbeFieldVerifierResult::Rejected => {
                field_admission_verdict(&reasons)
            }
            super::semantic_probe::ProbeFieldVerifierResult::Pending => {
                reasons.push("admission_not_completed".into());
                "rejected_invalid_state"
            }
        };
        (
            field.into(),
            TaskTruthFieldAdmissionV1 {
                verdict: verdict.into(),
                reasons,
            },
        )
    })
    .collect()
}

/// Atomically maps the locally admitted LCA-02 contract to the public answer.
/// Every compatibility field is derived from the same compact object. Local
/// recap, task-thread, and target wording are deliberately absent here.
pub(crate) fn map_compact_probe_output_to_public_answer(
    output: &super::semantic_probe::ProbeModelOutput,
    slots: &BTreeMap<String, super::semantic_probe::SupportSlot>,
    context: &CompactProbePublicMappingContext,
) -> TaskTruthPublicAnswerV1 {
    let support = |probe_field: &str, value_present: bool| {
        let evidence_refs = output
            .support_slots_by_field
            .get(probe_field)
            .into_iter()
            .flatten()
            .filter_map(|slot_id| slots.get(slot_id))
            .map(|slot| format!("{}:{}", slot.source_kind, slot.record_id))
            .collect::<Vec<_>>();
        let verifier_result = output
            .verifier_result_by_field
            .get(probe_field)
            .copied()
            .unwrap_or_default();
        let support_status = match verifier_result {
            super::semantic_probe::ProbeFieldVerifierResult::Admitted
                if evidence_refs.is_empty() =>
            {
                "partial"
            }
            super::semantic_probe::ProbeFieldVerifierResult::Admitted => "supported",
            super::semantic_probe::ProbeFieldVerifierResult::Rejected => "rejected",
            super::semantic_probe::ProbeFieldVerifierResult::NotProposed => "unsupported",
            super::semantic_probe::ProbeFieldVerifierResult::Pending if value_present => "partial",
            super::semantic_probe::ProbeFieldVerifierResult::Pending => "unsupported",
        };
        TaskTruthFieldSupportV1 {
            confidence: output.confidence_by_field.get(probe_field).copied(),
            support_status: support_status.into(),
            evidence_refs,
            missing_evidence: output
                .missing_evidence_by_field
                .get(probe_field)
                .cloned()
                .unwrap_or_default(),
            verifier_result: Some(verifier_result.label().into()),
        }
    };

    let semantic_support = BTreeMap::from([
        (
            "unfinished_task".to_string(),
            support("unfinished_task", output.unfinished_task.is_some()),
        ),
        (
            "task_state".to_string(),
            support(
                "task_state",
                output.task_state != super::semantic_probe::ProbeTaskState::Unclear,
            ),
        ),
        (
            "resume_point".to_string(),
            support("resume_point", output.resume_point.is_some()),
        ),
        (
            "next_supported_action".to_string(),
            support(
                "next_supported_action",
                output.next_supported_action.is_some(),
            ),
        ),
        (
            "completed_context".to_string(),
            support("completed_context", output.completed_context.is_some()),
        ),
        (
            "where_summary".to_string(),
            support("where_summary", output.where_summary.is_some()),
        ),
    ]);
    let mut field_support = semantic_support.clone();
    // Compatibility fields remain for existing consumers, but their support
    // and wording come only from the corresponding new semantic meaning.
    for (compatibility_field, semantic_field) in [
        ("task_summary", "unfinished_task"),
        ("execution_state", "task_state"),
        ("current_subtask", "resume_point"),
        ("unfinished_state", "resume_point"),
        ("last_meaningful_progress", "completed_context"),
        ("next_action", "next_supported_action"),
    ] {
        if let Some(value) = semantic_support.get(semantic_field) {
            field_support.insert(compatibility_field.into(), value.clone());
        }
    }

    let has_visible_semantics = output.unfinished_task.is_some()
        || output.task_state != super::semantic_probe::ProbeTaskState::Unclear
        || output.resume_point.is_some()
        || output.next_supported_action.is_some()
        || output.completed_context.is_some()
        || output.where_summary.is_some();
    let field_admission = compact_field_admission(output, &context.validation_issues);
    let admitted_semantic_status =
        compact_admitted_status(output, &context.semantic_conflicts).to_string();
    let public_next_supported_action =
        (!matches!(admitted_semantic_status.as_str(), "unresolved" | "refused"))
            .then(|| output.next_supported_action.clone())
            .flatten();
    let raw_model_status = probe_status_label(output.status).to_string();
    let task_supports = output
        .support_slots_by_field
        .get("unfinished_task")
        .cloned()
        .unwrap_or_default();
    let task_is_qualified = has_qualified_semantic_wording(output);
    let semantic_source_kind = if output.unfinished_task.is_none() {
        "unresolved"
    } else if !task_is_qualified
        && task_supports.iter().any(|slot| {
            context
                .explicit_goal_support_slots
                .iter()
                .any(|explicit| explicit == slot)
        })
    {
        "verified_cloud_explicit_goal"
    } else {
        "verified_cloud_inferred_goal"
    };
    let identity_seed = serde_json::json!({
        "packet_id": context.packet_id,
        "evidence_watermark": context.evidence_watermark,
        "provider_response_id": context.provider_response_id,
        "verifier_version": TASK_TRUTH_VERIFIER_VERSION,
        "admission_version": COMPACT_ADMISSION_VERSION,
        "response_schema_version": context.response_schema_version,
        "admitted_output": output,
        "admitted_semantic_status": admitted_semantic_status,
        "field_admission": field_admission,
        "semantic_conflicts": context.semantic_conflicts,
    });
    let identity_hash = stable_hash(identity_seed.to_string().as_bytes());
    let response_identity = context
        .provider_response_id
        .clone()
        .unwrap_or_else(|| format!("provider-response-envelope-{identity_hash}"));
    let selected_hypothesis_id =
        has_visible_semantics.then(|| format!("pftu-hypothesis-{identity_hash}"));
    let snapshot_id = format!("pftu-snapshot-{identity_hash}");
    let task_thread_id = has_visible_semantics.then(|| format!("pftu-task-thread-{identity_hash}"));
    let preview_frame_id = output
        .support_slots_by_field
        .values()
        .flatten()
        .filter_map(|slot_id| slots.get(slot_id))
        .find_map(|slot| slot.frame_id.clone());
    let target_status = if output.unfinished_task.is_none() {
        "no_task"
    } else if preview_frame_id.is_some() {
        "frame_preview_only"
    } else {
        "task_known_target_unknown"
    };
    let unresolved_or_failure_reason = if admitted_semantic_status == "resolved" {
        None
    } else if admitted_semantic_status == "refused" {
        Some("semantic_refused".to_string())
    } else if !context.semantic_conflicts.is_empty() {
        Some("p6_compact_semantic_conflict".to_string())
    } else if admitted_semantic_status == "unresolved" && !context.validation_issues.is_empty() {
        Some("support_validation_failure".to_string())
    } else if admitted_semantic_status == "unresolved" {
        Some("semantic_unresolved".to_string())
    } else if output.next_supported_action.is_none() {
        Some("missing_admitted_next_supported_action".to_string())
    } else if output.resume_point.is_none() {
        Some("missing_admitted_resume_point".to_string())
    } else if task_is_qualified {
        Some("qualified_semantic_claim".to_string())
    } else if raw_model_status == "partly_resolved" {
        Some("raw_model_partly_resolved".to_string())
    } else {
        Some("semantic_fields_incomplete".to_string())
    };
    let semantic_source = if has_visible_semantics {
        "cloud_multimodal_model"
    } else {
        "unresolved"
    };
    let current_relationship = context
        .recent_context
        .iter()
        .find(|visit| visit.is_current)
        .and_then(|visit| visit.semantic_role.clone())
        .unwrap_or_else(|| "unrelated_or_unknown".into());

    let mut answer = TaskTruthPublicAnswerV1 {
        task_basis: semantic_source_kind.into(),
        task_resolution_status: admitted_semantic_status.clone(),
        model_resolution_status: raw_model_status.clone(),
        raw_model_status,
        admitted_semantic_status,
        semantic_source_kind: semantic_source_kind.into(),
        field_admission,
        claim_confidence: output.confidence_by_field.clone(),
        target_status: target_status.into(),
        unresolved_or_failure_reason,
        semantic_conflicts: context.semantic_conflicts.clone(),
        atomic_answer_identity: identity_hash.clone(),
        unfinished_task: output.unfinished_task.clone(),
        task_state: output.task_state.label().into(),
        resume_point: output.resume_point.clone(),
        next_supported_action: public_next_supported_action.clone(),
        completed_context: output.completed_context.clone(),
        current_subtask: output.resume_point.clone(),
        current_activity: TaskTruthCurrentActivityV1 {
            current_subtask: output.resume_point.clone(),
            relationship_to_primary: current_relationship,
            ..Default::default()
        },
        task_summary: output.unfinished_task.clone(),
        last_meaningful_progress: output.completed_context.clone(),
        unfinished_state: output.resume_point.clone(),
        execution_state: output.task_state.label().into(),
        next_action: public_next_supported_action,
        where_summary: output.where_summary.clone(),
        recent_context: context.recent_context.clone(),
        field_support,
        direct_return_target: None,
        evidence_preview: preview_frame_id.map(|frame_id| ContinueEvidencePreview {
            schema: "smalltalk.continue_evidence_preview.v1".into(),
            preview_kind: "pftu_semantic_probe_evidence".into(),
            frame_id,
        }),
        task_understanding_source: semantic_source.into(),
        wording_source: "cloud_model".into(),
        target_selection_source: "strict_local_policy".into(),
        snapshot_id: snapshot_id.clone(),
        snapshot_revision: 1,
        evidence_watermark: context.evidence_watermark.clone(),
        semantic_source: semantic_source.into(),
        provider_name: Some("openai".into()),
        provider_model: context
            .response_model
            .clone()
            .or_else(|| Some(context.configured_model.clone())),
        request_id: context
            .provider_request_id
            .clone()
            .or_else(|| context.request_id.clone()),
        response_id: context.provider_response_id.clone(),
        selected_hypothesis_id: selected_hypothesis_id.clone(),
        inference_status: if context.validation_issues.is_empty() {
            context.diagnostic_status.clone()
        } else {
            "model_answer_visible_with_validation_limits".into()
        },
        atomic_identity: TaskTruthAtomicIdentityV1 {
            decision_id: context.decision_id.clone(),
            current_frame_id: context.current_frame_id.clone(),
            packet_policy_version: context.packet_policy_version.clone(),
            response_schema_version: context.response_schema_version.clone(),
            admission_version: COMPACT_ADMISSION_VERSION.into(),
            admitted_result_id: identity_hash,
            correction_watermark: context.correction_watermark.clone(),
            target_identity: None,
            session_id: context.session_id.clone(),
            task_thread_id,
            task_thread_revision: has_visible_semantics.then_some(1),
            task_snapshot_id: snapshot_id,
            snapshot_revision: 1,
            selected_hypothesis_id,
            model_request_id: context
                .provider_request_id
                .clone()
                .or_else(|| context.request_id.clone()),
            model_response_id: Some(response_identity),
            observation_packet_id: context.packet_id.clone(),
            evidence_watermark: context.evidence_watermark.clone(),
            correction_fingerprint: String::new(),
        },
        ..Default::default()
    };
    answer.recompute_product_projection();
    answer
}

#[allow(clippy::type_complexity)]
fn pftu_probe_public_result(
    conn: &Connection,
    decision_id: &str,
) -> Result<Option<PftuProbePublicResult>, String> {
    super::semantic_probe::ensure_schema(conn)?;
    let row = conn
        .query_row(
            "SELECT session_id, packet_id, evidence_watermark, model,
                    diagnostic_status, request_id, provider_request_id,
                    response_id, response_model, request_audit_json,
                    support_slot_map_json, admitted_output_json,
                    validation_issues_json, input_tokens, output_tokens,
                    total_tokens, estimated_cost_usd, latency_ms,
                    parsed_response, provider_post_count, failure_reason
             FROM task_truth_v2_semantic_probe_runs
             WHERE decision_id=?1
             ORDER BY created_at_ms DESC LIMIT 1",
            [decision_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, Option<i64>>(13)?,
                    row.get::<_, Option<i64>>(14)?,
                    row.get::<_, Option<i64>>(15)?,
                    row.get::<_, Option<f64>>(16)?,
                    row.get::<_, i64>(17)?,
                    row.get::<_, i64>(18)? != 0,
                    row.get::<_, i64>(19)?.max(0) as usize,
                    row.get::<_, Option<String>>(20)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((
        session_id,
        packet_id,
        evidence_watermark,
        configured_model,
        diagnostic_status,
        request_id,
        provider_request_id,
        provider_response_id,
        response_model,
        request_audit_json,
        support_slot_map_json,
        admitted_output_json,
        validation_issues_json,
        input_tokens,
        output_tokens,
        total_tokens,
        estimated_cost_usd,
        latency_ms,
        parsed_response,
        provider_post_count,
        failure_reason,
    )) = row
    else {
        return Ok(None);
    };

    let request_audit = request_audit_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<super::semantic_probe::ProbeRequestAudit>(raw).ok());
    let validation_issues =
        serde_json::from_str::<Vec<String>>(&validation_issues_json).unwrap_or_default();
    let output = probe_status_allows_admitted_output(&diagnostic_status)
        .then(|| {
            admitted_output_json.as_deref().and_then(|raw| {
                serde_json::from_str::<super::semantic_probe::ProbeModelOutput>(raw).ok()
            })
        })
        .flatten();
    let slots = support_slot_map_json
        .as_deref()
        .and_then(|raw| {
            serde_json::from_str::<BTreeMap<String, super::semantic_probe::SupportSlot>>(raw).ok()
        })
        .unwrap_or_default();
    let recent_context = request_audit
        .as_ref()
        .map(|audit| {
            audit
                .surface_timeline
                .iter()
                .map(|visit| {
                    let visit_id = if visit.visit_id.trim().is_empty() {
                        format!("T{}_VISIT", visit.sequence_index)
                    } else {
                        visit.visit_id.clone()
                    };
                    let role = output
                        .as_ref()
                        .and_then(|output| output.visit_roles.get(&visit_id));
                    TaskTruthRecentContextV1 {
                        sequence_index: visit.sequence_index,
                        app_label: if visit.private {
                            "Private activity".into()
                        } else {
                            visit.app_label.clone()
                        },
                        site_hostname: (!visit.private)
                            .then(|| visit.site_hostname.clone())
                            .flatten(),
                        first_observed_at_ms: visit.first_observed_at_ms,
                        last_observed_at_ms: visit.last_observed_at_ms,
                        is_current: visit.is_current,
                        revisited: visit.revisited,
                        evidence_refs: visit.evidence_refs.clone(),
                        semantic_role: role.map(|role| role.role.label().into()),
                        role_confidence: role.map(|role| role.confidence),
                        relationship_to_primary_task: role
                            .filter(|role| !role.relationship_to_primary_task.is_empty())
                            .map(|role| role.relationship_to_primary_task.clone()),
                        role_evidence_refs: role
                            .into_iter()
                            .flat_map(|role| role.support_slots.iter())
                            .filter_map(|slot_id| slots.get(slot_id))
                            .map(|slot| format!("{}:{}", slot.source_kind, slot.record_id))
                            .collect(),
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let legacy_compact_row = output.is_none()
        && admitted_output_json.as_deref().is_some_and(|raw| {
            serde_json::from_str::<Value>(raw)
                .ok()
                .and_then(|value| value.as_object().cloned())
                .is_some_and(|object| {
                    [
                        "primary_task",
                        "current_step",
                        "last_progress",
                        "unfinished_state",
                    ]
                    .iter()
                    .any(|field| object.contains_key(*field))
                })
        });
    let mapping_context = CompactProbePublicMappingContext {
        decision_id: decision_id.into(),
        session_id: session_id.clone(),
        packet_id: packet_id.clone(),
        evidence_watermark: evidence_watermark.clone(),
        configured_model: configured_model.clone(),
        response_model: response_model.clone(),
        request_id: request_id.clone(),
        provider_request_id: provider_request_id.clone(),
        provider_response_id: provider_response_id.clone(),
        diagnostic_status: diagnostic_status.clone(),
        validation_issues: validation_issues.clone(),
        recent_context: recent_context.clone(),
        current_frame_id: request_audit
            .as_ref()
            .map(|audit| audit.final_frame_id.clone())
            .unwrap_or_default(),
        packet_policy_version: request_audit
            .as_ref()
            .map(|audit| audit.request_schema.clone())
            .unwrap_or_default(),
        response_schema_version: super::semantic_probe::PROBE_RESPONSE_SCHEMA.into(),
        explicit_goal_support_slots: request_audit
            .as_ref()
            .map(|audit| audit.explicit_goal_support_slots.clone())
            .unwrap_or_default(),
        correction_watermark: String::new(),
        semantic_conflicts: validation_issues
            .iter()
            .filter(|issue| issue.starts_with("p6_compact_"))
            .cloned()
            .collect(),
    };
    let answer = output
        .as_ref()
        .map(|output| map_compact_probe_output_to_public_answer(output, &slots, &mapping_context));
    let unresolved_identity_seed = serde_json::json!({
        "decision_id": decision_id,
        "packet_id": packet_id,
        "request_id": provider_request_id.as_ref().or(request_id.as_ref()),
        "provider_response_id": provider_response_id,
        "legacy_compact_row": legacy_compact_row,
    });
    let unresolved_identity_hash = stable_hash(unresolved_identity_seed.to_string().as_bytes());
    let unresolved_snapshot_id = format!("pftu-snapshot-{unresolved_identity_hash}");
    let unresolved_response_identity = provider_response_id
        .clone()
        .unwrap_or_else(|| format!("provider-response-envelope-{unresolved_identity_hash}"));
    let answer = answer.or_else(|| {
        (!recent_context.is_empty() || legacy_compact_row || failure_reason.is_some()).then(|| {
            TaskTruthPublicAnswerV1 {
                task_resolution_status: "unresolved".into(),
                model_resolution_status: "unresolved".into(),
                unresolved_or_failure_reason: failure_reason
                    .clone()
                    .or_else(|| Some(diagnostic_status.clone())),
                recent_context: recent_context.clone(),
                task_understanding_source: "unresolved".into(),
                wording_source: "deterministic".into(),
                target_selection_source: "strict_local_policy".into(),
                snapshot_id: unresolved_snapshot_id.clone(),
                snapshot_revision: 1,
                evidence_watermark: evidence_watermark.clone(),
                semantic_source: "unresolved".into(),
                provider_name: Some("openai".into()),
                provider_model: response_model.clone().or(Some(configured_model.clone())),
                request_id: provider_request_id.clone().or(request_id.clone()),
                response_id: provider_response_id.clone(),
                inference_status: if legacy_compact_row {
                    "legacy_compact_contract_downgraded".into()
                } else {
                    diagnostic_status.clone()
                },
                atomic_identity: TaskTruthAtomicIdentityV1 {
                    session_id: session_id.clone(),
                    task_thread_id: None,
                    task_thread_revision: None,
                    task_snapshot_id: unresolved_snapshot_id.clone(),
                    snapshot_revision: 1,
                    selected_hypothesis_id: None,
                    model_request_id: provider_request_id.clone().or(request_id.clone()),
                    model_response_id: Some(unresolved_response_identity.clone()),
                    observation_packet_id: packet_id.clone(),
                    evidence_watermark: evidence_watermark.clone(),
                    correction_fingerprint: String::new(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
    });
    let has_visible_semantics = answer.as_ref().is_some_and(|answer| {
        answer.unfinished_task.is_some()
            || answer.task_state != "unclear"
            || answer.resume_point.is_some()
            || answer.next_supported_action.is_some()
            || answer.completed_context.is_some()
            || answer.where_summary.is_some()
    });

    let diagnostic = TaskTruthInferenceDiagnosticV1 {
        schema: "smalltalk.task_truth_inference_diagnostic.v1".into(),
        status: if legacy_compact_row {
            "legacy_compact_contract_downgraded".into()
        } else if has_visible_semantics {
            "success".into()
        } else {
            diagnostic_status.clone()
        },
        origin: if parsed_response
            || provider_request_id.is_some()
            || provider_response_id.is_some()
        {
            "live_cloud"
        } else {
            "none"
        }
        .into(),
        provider: "openai".into(),
        model: response_model.unwrap_or(configured_model),
        request_id,
        provider_request_id,
        response_id: provider_response_id,
        provider_attempt_count: provider_post_count,
        latency_ms,
        image_count: request_audit
            .as_ref()
            .map(|audit| audit.image_count)
            .unwrap_or(0),
        image_bytes: request_audit
            .as_ref()
            .map(|audit| audit.image_bytes)
            .unwrap_or(0),
        estimated_tokens: request_audit
            .as_ref()
            .map(|audit| audit.estimated_text_tokens)
            .unwrap_or(0),
        input_tokens,
        output_tokens,
        total_tokens,
        estimated_cost_usd,
        verification_status: if validation_issues.is_empty() {
            "accepted"
        } else {
            "partially_accepted"
        }
        .into(),
        selected_hypothesis_id: answer
            .as_ref()
            .and_then(|answer| answer.selected_hypothesis_id.clone()),
    };
    Ok(Some(PftuProbePublicResult { answer, diagnostic }))
}

#[allow(dead_code)]
fn visible_legacy_model_answer(
    conn: &Connection,
    decision_id: &str,
) -> Result<Option<TaskTruthPublicAnswerV1>, String> {
    let raw = conn
        .query_row(
            "SELECT packet_summary_json FROM task_truth_v2_shadow_audits
             WHERE decision_id=?1 ORDER BY observed_at_ms DESC LIMIT 1",
            [decision_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some(raw) = raw else {
        return Ok(None);
    };
    let packet_summary: Value = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    let Some(multimodal_value) = packet_summary.get("multimodal") else {
        return Ok(None);
    };
    let Ok(multimodal) =
        serde_json::from_value::<super::MultimodalShadowAuditV1>(multimodal_value.clone())
    else {
        // Historical and test diagnostics may predate the current full audit
        // shape. A diagnostic projection must never make Continue fail or
        // synthesize semantics from a partially decoded row.
        return Ok(None);
    };
    let resolver = &multimodal.resolver;
    let Some(output) = resolver.output.as_ref() else {
        return Ok(None);
    };
    let selected = output.hypotheses.iter().max_by(|left, right| {
        left.confidence
            .partial_cmp(&right.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.hypothesis_id.cmp(&left.hypothesis_id))
    });
    let Some(selected) = selected else {
        return Ok(None);
    };
    let has_visible_semantics = selected.likely_primary_task.is_some()
        || selected.task_object.is_some()
        || selected.observed_surface.is_some()
        || selected.app_identity.is_some()
        || selected.current_subtask.is_some()
        || selected.last_meaningful_progress.is_some()
        || selected.unfinished_state.is_some()
        || selected.possible_next_action.is_some()
        || selected.immediate_user_operation.is_some()
        || selected.semantic_effect_of_operation.is_some();
    if !has_visible_semantics {
        return Ok(None);
    }

    let packet_id = packet_summary
        .get("packet_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown-packet")
        .to_string();
    let evidence_watermark = packet_summary
        .get("evidence_watermark")
        .and_then(Value::as_str)
        .unwrap_or("unknown-watermark")
        .to_string();
    let session_id = packet_summary
        .get("session_id")
        .and_then(Value::as_str)
        .map(str::to_string);
    let identity_seed = serde_json::json!({
        "decision_id": decision_id,
        "packet_id": packet_id,
        "provider_response_id": resolver.response_id,
        "selected_hypothesis": selected,
    });
    let identity_hash = stable_hash(identity_seed.to_string().as_bytes());
    let response_identity = resolver
        .response_id
        .clone()
        .unwrap_or_else(|| format!("provider-response-envelope-{identity_hash}"));
    let snapshot_id = format!("visible-model-snapshot-{identity_hash}");
    let selected_hypothesis_id = if selected.hypothesis_id.trim().is_empty() {
        format!("visible-model-hypothesis-{identity_hash}")
    } else {
        selected.hypothesis_id.clone()
    };

    let field_support = |model_field: &str| {
        let claim = selected
            .claim_evidence
            .get(model_field)
            .and_then(Option::as_ref);
        TaskTruthFieldSupportV1 {
            confidence: selected
                .confidence_by_field
                .get(model_field)
                .copied()
                .or(Some(selected.confidence)),
            support_status: if claim.is_some() {
                "model_returned_verification_limited"
            } else {
                "model_returned_without_local_support"
            }
            .into(),
            evidence_refs: claim
                .into_iter()
                .flat_map(|claim| claim.evidence_refs.iter())
                .map(|evidence| format!("{}:{}", evidence.source_kind, evidence.record_id))
                .collect(),
            missing_evidence: Vec::new(),
            verifier_result: None,
        }
    };
    let mut support = BTreeMap::new();
    for (public_field, model_field) in [
        ("task_summary", "likely_primary_task"),
        ("task_object", "task_object"),
        ("last_meaningful_progress", "last_meaningful_progress"),
        ("unfinished_state", "unfinished_state"),
        ("next_action", "possible_next_action"),
        ("observed_surface", "observed_surface"),
        ("immediate_user_operation", "immediate_user_operation"),
        (
            "semantic_effect_of_operation",
            "semantic_effect_of_operation",
        ),
        ("current_subtask", "current_subtask"),
        ("where_summary", "app_identity"),
    ] {
        support.insert(public_field.into(), field_support(model_field));
    }
    let evidence_preview = selected
        .claim_evidence
        .values()
        .filter_map(Option::as_ref)
        .flat_map(|claim| claim.evidence_refs.iter())
        .find_map(|evidence| evidence.frame_id.clone())
        .or_else(|| {
            packet_summary
                .get("current_frame_id")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .map(|frame_id| ContinueEvidencePreview {
            schema: "smalltalk.continue_evidence_preview.v1".into(),
            preview_kind: "model_answer_verification_limited_evidence".into(),
            frame_id,
        });
    let mut alternatives = output
        .hypotheses
        .iter()
        .filter(|hypothesis| hypothesis.hypothesis_id != selected.hypothesis_id)
        .filter_map(|hypothesis| {
            hypothesis
                .likely_primary_task
                .as_ref()
                .map(|task_summary| TaskTruthAlternativeV1 {
                    hypothesis_id: hypothesis.hypothesis_id.clone(),
                    task_summary: task_summary.clone(),
                    relation: hypothesis.relationship_to_prior.label().into(),
                    confidence: hypothesis.confidence,
                    disposition: "model_returned_verification_limited".into(),
                    ..Default::default()
                })
        })
        .collect::<Vec<_>>();
    alternatives.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    alternatives.truncate(2);

    Ok(Some(TaskTruthPublicAnswerV1 {
        task_basis: if selected.likely_primary_task.is_some() {
            "verified_cloud_inferred_goal"
        } else {
            "unresolved"
        }
        .into(),
        task_resolution_status: if selected.likely_primary_task.is_some() {
            "partly_resolved"
        } else {
            "unresolved"
        }
        .into(),
        admitted_semantic_status: if selected.likely_primary_task.is_some() {
            "partly_resolved"
        } else {
            "unresolved"
        }
        .into(),
        semantic_source_kind: if selected.likely_primary_task.is_some() {
            "verified_cloud_inferred_goal"
        } else {
            "unresolved"
        }
        .into(),
        observed_surface: selected.observed_surface.clone(),
        immediate_user_operation: selected.immediate_user_operation.clone(),
        semantic_effect_of_operation: selected.semantic_effect_of_operation.clone(),
        current_subtask: selected.current_subtask.clone(),
        current_activity: TaskTruthCurrentActivityV1 {
            observed_surface: selected.observed_surface.clone(),
            immediate_user_operation: selected.immediate_user_operation.clone(),
            semantic_effect_of_operation: selected.semantic_effect_of_operation.clone(),
            current_subtask: selected.current_subtask.clone(),
            relationship_to_primary: selected.relationship_to_prior.label().into(),
        },
        task_summary: selected.likely_primary_task.clone(),
        task_object: selected.task_object.clone(),
        last_meaningful_progress: selected.last_meaningful_progress.clone(),
        unfinished_state: selected.unfinished_state.clone(),
        execution_state: selected
            .execution_state
            .clone()
            .unwrap_or_else(|| "unclear".into()),
        next_action: selected.possible_next_action.clone(),
        where_summary: selected.app_identity.clone(),
        relationship_to_prior: selected.relationship_to_prior.label().into(),
        alternative_hypotheses: alternatives,
        direct_return_target: None,
        evidence_preview,
        field_support: support,
        task_understanding_source: "cloud_multimodal_model".into(),
        wording_source: "cloud_model".into(),
        target_selection_source: "strict_local_policy".into(),
        snapshot_id: snapshot_id.clone(),
        snapshot_revision: 1,
        evidence_watermark: evidence_watermark.clone(),
        semantic_source: "cloud_multimodal_model".into(),
        provider_name: Some(resolver.provider.clone()),
        provider_model: Some(resolver.model.clone()),
        request_id: resolver
            .provider_request_id
            .clone()
            .or_else(|| resolver.request_id.clone()),
        response_id: resolver.response_id.clone(),
        selected_hypothesis_id: Some(selected_hypothesis_id.clone()),
        inference_status: "model_answer_visible_with_verification_limits".into(),
        atomic_identity: TaskTruthAtomicIdentityV1 {
            session_id,
            task_thread_id: Some(format!("visible-model-task-thread-{identity_hash}")),
            task_thread_revision: Some(1),
            task_snapshot_id: snapshot_id,
            snapshot_revision: 1,
            selected_hypothesis_id: Some(selected_hypothesis_id),
            model_request_id: resolver
                .provider_request_id
                .clone()
                .or_else(|| resolver.request_id.clone()),
            model_response_id: Some(response_identity),
            observation_packet_id: packet_id,
            evidence_watermark,
            correction_fingerprint: String::new(),
            ..Default::default()
        },
        ..Default::default()
    }))
}

fn typed_unresolved_answer(
    session_id: Option<&str>,
    diagnostic: Option<&TaskTruthInferenceDiagnosticV1>,
    boundary_snapshot: Option<&TaskSnapshotV2>,
    boundary_packet_id: Option<&str>,
) -> TaskTruthPublicAnswerV1 {
    let inference_status = diagnostic
        .map(|diagnostic| diagnostic.status.clone())
        .unwrap_or_else(|| "no_verified_snapshot".into());
    let observation_packet_id = boundary_packet_id
        .or_else(|| boundary_snapshot.map(|snapshot| snapshot.packet_id.as_str()))
        .unwrap_or_default()
        .to_string();
    let task_snapshot_id = boundary_snapshot
        .map(|snapshot| snapshot.snapshot_id.clone())
        .unwrap_or_default();
    let snapshot_revision = boundary_snapshot
        .map(|snapshot| snapshot.revision)
        .unwrap_or_default();
    let evidence_watermark = boundary_snapshot
        .map(|snapshot| snapshot.evidence_watermark.clone())
        .unwrap_or_default();
    let mut answer = TaskTruthPublicAnswerV1 {
        unresolved_or_failure_reason: Some(inference_status.clone()),
        inference_status,
        provider_name: diagnostic.map(|diagnostic| diagnostic.provider.clone()),
        provider_model: diagnostic.map(|diagnostic| diagnostic.model.clone()),
        request_id: diagnostic.and_then(|diagnostic| diagnostic.request_id.clone()),
        response_id: diagnostic.and_then(|diagnostic| diagnostic.response_id.clone()),
        snapshot_id: task_snapshot_id.clone(),
        snapshot_revision,
        evidence_watermark: evidence_watermark.clone(),
        atomic_identity: TaskTruthAtomicIdentityV1 {
            session_id: boundary_snapshot
                .and_then(|snapshot| snapshot.session_id.clone())
                .or_else(|| session_id.map(str::to_string)),
            task_snapshot_id,
            snapshot_revision,
            model_request_id: diagnostic.and_then(|diagnostic| diagnostic.request_id.clone()),
            model_response_id: diagnostic.and_then(|diagnostic| diagnostic.response_id.clone()),
            observation_packet_id,
            evidence_watermark,
            ..Default::default()
        },
        ..Default::default()
    };
    answer.recompute_product_projection();
    answer
}

fn field_support(snapshot: &TaskSnapshotV2, field: &str) -> TaskTruthFieldSupportV1 {
    let confidence = snapshot.confidence_by_field.get(field).copied();
    let evidence_refs = snapshot
        .claim_evidence
        .iter()
        .filter(|claim| claim.claim == field)
        .flat_map(|claim| claim.evidence_refs.iter())
        .map(|evidence| format!("{}:{}", evidence.source_kind, evidence.record_id))
        .collect::<Vec<_>>();
    TaskTruthFieldSupportV1 {
        confidence,
        support_status: if !evidence_refs.is_empty() && confidence.unwrap_or(0.0) > 0.0 {
            "supported".into()
        } else if confidence.is_some() {
            "partial".into()
        } else {
            "unsupported".into()
        },
        evidence_refs,
        missing_evidence: Vec::new(),
        verifier_result: None,
    }
}

fn understanding_source(snapshot: &TaskSnapshotV2) -> String {
    snapshot.semantic_source.clone()
}

fn public_answer(snapshot: &TaskSnapshotV2) -> TaskTruthPublicAnswerV1 {
    let mut field_support_map = BTreeMap::new();
    for (public_field, verified_field) in [
        ("task_summary", "likely_primary_task"),
        ("task_object", "task_object"),
        ("execution_state", "execution_state"),
        ("last_meaningful_progress", "last_meaningful_progress"),
        ("unfinished_state", "unfinished_state"),
        ("next_action", "possible_next_action"),
        ("observed_surface", "observed_surface"),
        ("immediate_user_operation", "immediate_user_operation"),
        (
            "semantic_effect_of_operation",
            "semantic_effect_of_operation",
        ),
        ("current_subtask", "current_subtask"),
    ] {
        field_support_map.insert(public_field.into(), field_support(snapshot, verified_field));
    }
    field_support_map.insert(
        "where_summary".into(),
        field_support(snapshot, "app_identity"),
    );
    let alternatives = snapshot
        .alternative_hypotheses
        .iter()
        .take(2)
        .map(|hypothesis| TaskTruthAlternativeV1 {
            hypothesis_id: if hypothesis.hypothesis_id.is_empty() {
                format!(
                    "hypothesis-{}",
                    stable_hash(
                        format!("{}:{}", snapshot.snapshot_id, hypothesis.summary).as_bytes()
                    )
                )
            } else {
                hypothesis.hypothesis_id.clone()
            },
            task_summary: hypothesis.summary.clone(),
            relation: hypothesis.relation.clone(),
            confidence: hypothesis.confidence,
            evidence_refs: hypothesis
                .evidence_refs
                .iter()
                .map(|evidence| format!("{}:{}", evidence.source_kind, evidence.record_id))
                .collect(),
            contradicting_evidence_refs: hypothesis
                .contradicting_evidence_refs
                .iter()
                .map(|evidence| format!("{}:{}", evidence.source_kind, evidence.record_id))
                .collect(),
            task_thread_id: hypothesis.task_thread_id.clone(),
            task_thread_revision: hypothesis.task_thread_revision,
            last_supported_at_ms: hypothesis.last_supported_at_ms,
            disposition: hypothesis.disposition.clone(),
            reason_codes: hypothesis.reason_codes.clone(),
            semantic_payload: hypothesis.semantic_payload.clone(),
        })
        .collect::<Vec<_>>();
    let task_resolution_status = if snapshot.task_summary.is_none() {
        "unresolved"
    } else if !alternatives.is_empty() {
        "ambiguous"
    } else {
        "resolved"
    };
    let preview_ref = snapshot
        .claim_evidence
        .iter()
        .flat_map(|claim| claim.evidence_refs.iter())
        .find_map(|evidence| evidence.frame_id.clone());
    let mut answer = TaskTruthPublicAnswerV1 {
        schema: TASK_TRUTH_PUBLIC_ANSWER_SCHEMA_V1.into(),
        task_basis: snapshot.task_basis.clone(),
        task_resolution_status: task_resolution_status.into(),
        model_resolution_status: "unresolved".into(),
        raw_model_status: "unresolved".into(),
        admitted_semantic_status: task_resolution_status.into(),
        semantic_source_kind: snapshot.task_basis.clone(),
        field_admission: BTreeMap::new(),
        claim_confidence: snapshot.confidence_by_field.clone(),
        target_status: if preview_ref.is_some() {
            "frame_preview_only"
        } else if snapshot.task_summary.is_some() {
            "task_known_target_unknown"
        } else {
            "no_task"
        }
        .into(),
        unresolved_or_failure_reason: (task_resolution_status == "unresolved")
            .then(|| "semantic_unresolved".into()),
        semantic_conflicts: snapshot.contradictions.clone(),
        atomic_answer_identity: stable_hash(
            format!("{}:{}", snapshot.snapshot_id, snapshot.revision).as_bytes(),
        ),
        product_projection: ContinueProductProjectionV1::default(),
        stale_product_projection: default_stale_product_projection(),
        unfinished_task: None,
        task_state: "unclear".into(),
        resume_point: None,
        next_supported_action: None,
        completed_context: None,
        observed_surface: snapshot.observed_surface.clone(),
        immediate_user_operation: snapshot.immediate_user_operation.clone(),
        semantic_effect_of_operation: snapshot.semantic_effect_of_operation.clone(),
        current_subtask: snapshot.current_subtask.clone(),
        current_activity: TaskTruthCurrentActivityV1 {
            observed_surface: snapshot.observed_surface.clone(),
            immediate_user_operation: snapshot.immediate_user_operation.clone(),
            semantic_effect_of_operation: snapshot.semantic_effect_of_operation.clone(),
            current_subtask: snapshot.current_subtask.clone(),
            relationship_to_primary: snapshot.relation_to_prior.clone(),
        },
        task_summary: snapshot.task_summary.clone(),
        task_object: snapshot.task_object.clone(),
        last_meaningful_progress: snapshot.last_meaningful_progress.clone(),
        unfinished_state: snapshot.unfinished_step.clone(),
        recent_context: Vec::new(),
        execution_state: snapshot.execution_state.clone(),
        next_action: snapshot.next_action.clone(),
        where_summary: snapshot.app_identity.clone(),
        relationship_to_prior: snapshot.relation_to_prior.clone(),
        alternative_hypotheses: alternatives,
        direct_return_target: None,
        evidence_preview: preview_ref.map(|frame_id| ContinueEvidencePreview {
            schema: "smalltalk.continue_evidence_preview.v1".into(),
            preview_kind: "task_snapshot_evidence".into(),
            frame_id,
        }),
        field_support: field_support_map,
        task_understanding_source: understanding_source(snapshot),
        wording_source: "deterministic".into(),
        target_selection_source: "strict_local_policy".into(),
        snapshot_id: snapshot.snapshot_id.clone(),
        snapshot_revision: snapshot.revision,
        evidence_watermark: snapshot.evidence_watermark.clone(),
        semantic_source: snapshot.semantic_source.clone(),
        provider_name: snapshot.provider_name.clone(),
        provider_model: snapshot.provider_model.clone(),
        request_id: snapshot.provider_request_id.clone(),
        response_id: snapshot.provider_response_id.clone(),
        selected_hypothesis_id: snapshot.selected_hypothesis_id.clone(),
        inference_status: snapshot.inference_status.clone(),
        atomic_identity: TaskTruthAtomicIdentityV1 {
            session_id: snapshot.session_id.clone(),
            task_thread_id: snapshot.task_thread_id.clone(),
            task_thread_revision: snapshot.task_thread_revision,
            task_snapshot_id: snapshot.snapshot_id.clone(),
            snapshot_revision: snapshot.revision,
            selected_hypothesis_id: snapshot.selected_hypothesis_id.clone(),
            model_request_id: snapshot.provider_request_id.clone(),
            model_response_id: snapshot.provider_response_id.clone(),
            observation_packet_id: snapshot.packet_id.clone(),
            evidence_watermark: snapshot.evidence_watermark.clone(),
            correction_fingerprint: String::new(),
            ..Default::default()
        },
    };
    answer.recompute_product_projection();
    answer
}

fn apply_scoped_feedback(
    conn: &Connection,
    answer: &mut TaskTruthPublicAnswerV1,
) -> Result<(), String> {
    let mut statement = conn
        .prepare(
            "SELECT feedback_id, affected_field, hypothesis_id, feedback_kind, correction_value
             FROM task_truth_v2_feedback_events
             WHERE task_snapshot_id=?1 AND task_snapshot_revision=?2
             ORDER BY observed_at_ms ASC, feedback_id ASC",
        )
        .map_err(|error| error.to_string())?;
    let feedback = statement
        .query_map(
            params![answer.snapshot_id, answer.snapshot_revision],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;

    let correction_fingerprint = stable_hash(
        serde_json::to_string(&feedback)
            .map_err(|error| error.to_string())?
            .as_bytes(),
    );
    if !feedback.is_empty() {
        answer.atomic_identity.correction_fingerprint = correction_fingerprint.clone();
        answer.atomic_identity.correction_watermark = correction_fingerprint;
    }
    for (_feedback_id, field, hypothesis_id, kind, correction_value) in feedback {
        if field == "hypothesis" {
            let Some(hypothesis_id) = hypothesis_id else {
                continue;
            };
            if kind == "corrected" {
                if let Some(selected) = answer
                    .alternative_hypotheses
                    .iter()
                    .find(|hypothesis| hypothesis.hypothesis_id == hypothesis_id)
                    .cloned()
                {
                    let previous = TaskTruthAlternativeV1 {
                        hypothesis_id: answer
                            .selected_hypothesis_id
                            .clone()
                            .unwrap_or_else(|| "previous_selected_hypothesis".into()),
                        task_summary: answer.task_summary.clone().unwrap_or_default(),
                        relation: answer.relationship_to_prior.clone(),
                        confidence: answer
                            .field_support
                            .get("task_summary")
                            .and_then(|support| support.confidence)
                            .unwrap_or(0.0),
                        evidence_refs: answer
                            .field_support
                            .get("task_summary")
                            .map(|support| support.evidence_refs.clone())
                            .unwrap_or_default(),
                        disposition: "demoted_by_human_choice".into(),
                        reason_codes: vec!["human_selected_competing_hypothesis".into()],
                        ..Default::default()
                    };
                    if let Some(payload) = selected.semantic_payload.clone() {
                        if let Ok(model) =
                            serde_json::from_value::<super::model::ModelTaskHypothesisV1>(payload)
                        {
                            answer.observed_surface = model.observed_surface.clone();
                            answer.immediate_user_operation =
                                model.immediate_user_operation.clone();
                            answer.semantic_effect_of_operation =
                                model.semantic_effect_of_operation.clone();
                            answer.current_subtask = model.current_subtask.clone();
                            answer.current_activity = TaskTruthCurrentActivityV1 {
                                observed_surface: model.observed_surface.clone(),
                                immediate_user_operation: model.immediate_user_operation.clone(),
                                semantic_effect_of_operation: model
                                    .semantic_effect_of_operation
                                    .clone(),
                                current_subtask: model.current_subtask.clone(),
                                relationship_to_primary: model.relationship_to_prior.label().into(),
                            };
                            answer.task_summary = model.likely_primary_task.clone();
                            answer.task_object = model.task_object.clone();
                            answer.last_meaningful_progress =
                                model.last_meaningful_progress.clone();
                            answer.unfinished_state = model.unfinished_state.clone();
                            answer.execution_state =
                                model.execution_state.unwrap_or_else(|| "unclear".into());
                            answer.next_action = model.possible_next_action.clone();
                            answer.where_summary = model.app_identity.clone();
                            answer.relationship_to_prior =
                                model.relationship_to_prior.label().into();
                        }
                    } else {
                        answer.task_summary = Some(selected.task_summary.clone());
                    }
                    answer.task_resolution_status = "resolved".into();
                    answer.task_understanding_source = "human_correction".into();
                    answer.semantic_source = "human_correction".into();
                    answer.semantic_source_kind = "human_correction".into();
                    answer.selected_hypothesis_id = Some(hypothesis_id.clone());
                    answer.atomic_identity.selected_hypothesis_id = Some(hypothesis_id.clone());
                    answer.field_support.insert(
                        "task_summary".into(),
                        TaskTruthFieldSupportV1 {
                            confidence: Some(1.0),
                            support_status: "human_corrected".into(),
                            evidence_refs: selected.evidence_refs,
                            missing_evidence: Vec::new(),
                            verifier_result: Some("human_corrected".into()),
                        },
                    );
                    answer
                        .alternative_hypotheses
                        .retain(|hypothesis| hypothesis.hypothesis_id != hypothesis_id);
                    if !previous.task_summary.trim().is_empty()
                        && previous.hypothesis_id != hypothesis_id
                    {
                        answer.alternative_hypotheses.push(previous);
                        answer.alternative_hypotheses.truncate(2);
                    }
                }
            } else if kind == "rejected" {
                answer
                    .alternative_hypotheses
                    .retain(|hypothesis| hypothesis.hypothesis_id != hypothesis_id);
            }
            continue;
        }
        if field == "relationship" {
            if let Some(value) = correction_value {
                answer.relationship_to_prior = value.clone();
                answer.current_activity.relationship_to_primary = value;
                answer.task_understanding_source = "human_correction".into();
                answer.semantic_source = "human_correction".into();
                answer.semantic_source_kind = "human_correction".into();
            }
            continue;
        }
        if field == "task_status" {
            if let Some(value) = correction_value {
                answer.execution_state = if value == "reactivated" {
                    "active".into()
                } else {
                    value
                };
                answer.task_state = answer.execution_state.clone();
                answer.task_understanding_source = "human_correction".into();
                answer.semantic_source = "human_correction".into();
                answer.semantic_source_kind = "human_correction".into();
            }
            continue;
        }
        if kind != "rejected" {
            continue;
        }
        let support_key = match field.as_str() {
            "state" => "execution_state",
            "where" => "where_summary",
            other => other,
        };
        answer
            .field_support
            .entry(support_key.into())
            .and_modify(|support| support.support_status = "rejected_by_user".into())
            .or_insert_with(|| TaskTruthFieldSupportV1 {
                confidence: None,
                support_status: "rejected_by_user".into(),
                evidence_refs: Vec::new(),
                missing_evidence: Vec::new(),
                verifier_result: Some("rejected_by_user".into()),
            });
        answer.field_admission.insert(
            support_key.into(),
            TaskTruthFieldAdmissionV1 {
                verdict: "rejected_unsupported".into(),
                reasons: vec!["human_rejected_exact_field".into()],
            },
        );
        match field.as_str() {
            "task_summary" => {
                answer.task_resolution_status = "unresolved".into();
                answer.admitted_semantic_status = "unresolved".into();
                answer.unfinished_task = None;
                answer.task_summary = None;
                answer.task_object = None;
                answer.direct_return_target = None;
                answer.target_status = "no_task".into();
                answer.unresolved_or_failure_reason = Some("human_rejected_unfinished_task".into());
                answer.task_understanding_source = "unresolved".into();
            }
            "task_object" => answer.task_object = None,
            "state" => {
                answer.task_state = "unclear".into();
                answer.resume_point = None;
                answer.completed_context = None;
                answer.last_meaningful_progress = None;
                answer.unfinished_state = None;
            }
            "next_action" => {
                answer.next_supported_action = None;
                answer.next_action = None;
                if answer.unfinished_task.is_some() {
                    answer.task_resolution_status = "partly_resolved".into();
                    answer.admitted_semantic_status = "partly_resolved".into();
                    answer.unresolved_or_failure_reason =
                        Some("human_rejected_next_supported_action".into());
                }
            }
            "where" => {
                answer.where_summary = None;
                answer.direct_return_target = None;
                answer.target_status = if answer.evidence_preview.is_some() {
                    "frame_preview_only"
                } else if answer.unfinished_task.is_some() || answer.task_summary.is_some() {
                    "task_known_target_unknown"
                } else {
                    "no_task"
                }
                .into();
            }
            _ => {}
        }
    }
    answer.recompute_product_projection();
    Ok(())
}

fn ensure_authority_audit_schema(conn: &Connection) -> Result<(), String> {
    checkpoint::ensure_schema(conn)
}

fn audit_switch(conn: &Connection, decision: &TaskTruthProductionDecisionV1) -> Result<(), String> {
    ensure_authority_audit_schema(conn)?;
    let prior = conn
        .query_row(
            "SELECT effective_state FROM task_truth_v2_authority_audits
             ORDER BY observed_at_ms DESC, audit_id DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if prior.as_deref() == Some(decision.effective_state.label()) {
        return Ok(());
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default();
    let audit_id = format!(
        "tt2-authority-{}",
        stable_hash(
            format!(
                "{now}:{}:{}",
                decision.requested_state.label(),
                decision.effective_state.label()
            )
            .as_bytes()
        )
    );
    conn.execute(
        "INSERT INTO task_truth_v2_authority_audits (
           audit_id, observed_at_ms, requested_state, effective_state,
           release_gate_passed, policy_version, cache_fingerprint, reason_codes_json
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            audit_id,
            now,
            decision.requested_state.label(),
            decision.effective_state.label(),
            i64::from(decision.release_gate_passed),
            decision.policy_version,
            decision.cache_fingerprint,
            serde_json::to_string(&decision.reason_codes).map_err(|error| error.to_string())?,
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub(crate) fn production_decision(
    conn: &Connection,
    session_id: Option<&str>,
    model_first_answer_required: bool,
) -> Result<TaskTruthProductionDecisionV1, String> {
    production_decision_for_attempt(conn, session_id, model_first_answer_required, None)
}

pub(crate) fn production_decision_for_attempt(
    conn: &Connection,
    session_id: Option<&str>,
    model_first_answer_required: bool,
    decision_id: Option<&str>,
) -> Result<TaskTruthProductionDecisionV1, String> {
    let requested_state = TaskTruthAuthorityStateV1::from_environment();
    let (release_gate_passed, release_gate_source, mut reason_codes) = read_release_gate();
    let effective_state = match requested_state {
        TaskTruthAuthorityStateV1::Authoritative if release_gate_passed => {
            TaskTruthAuthorityStateV1::Authoritative
        }
        TaskTruthAuthorityStateV1::Authoritative => {
            reason_codes.push("authoritative_blocked_by_release_gate".into());
            TaskTruthAuthorityStateV1::Eligible
        }
        other => other,
    };
    // An absent scope is an explicit unresolved state. It must never promote an
    // old snapshot merely because that snapshot is the newest row in the database.
    let snapshots = if decision_id.is_some() {
        // The manual attempt below has its own decision-bound packet and
        // snapshot identity. Do not run session-wide selection in parallel.
        Vec::new()
    } else if session_id.is_some() {
        checkpoint::load_recent_snapshots(conn, session_id, 24)?
    } else {
        reason_codes.push("unscoped_request_no_snapshot_selection".into());
        Vec::new()
    };
    let selection = super::selection::select_snapshot(&snapshots);
    let selected = selection
        .selected_snapshot_id
        .as_deref()
        .and_then(|id| snapshots.iter().find(|snapshot| snapshot.snapshot_id == id));
    if decision_id.is_none() && selected.is_none() {
        reason_codes.push("no_verified_snapshot_selected".into());
    }
    let newest = snapshots.iter().max_by(|left, right| {
        left.observed_at_ms
            .cmp(&right.observed_at_ms)
            .then_with(|| left.revision.cmp(&right.revision))
    });
    let attempt_identity = decision_id
        .map(|decision_id| {
            conn.query_row(
                "SELECT packet_id, selected_snapshot_id FROM task_truth_v2_shadow_audits
                 WHERE decision_id=?1 ORDER BY observed_at_ms DESC LIMIT 1",
                params![decision_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()
            .map_err(|error| error.to_string())
        })
        .transpose()?
        .flatten();
    let attempt_packet_id = attempt_identity
        .as_ref()
        .map(|(packet_id, _)| packet_id.as_str());
    let attempt_snapshot = attempt_identity
        .as_ref()
        .map(|(packet_id, selected_snapshot_id)| {
            if let Some(snapshot_id) = selected_snapshot_id.as_deref() {
                conn.query_row(
                    "SELECT snapshot_json FROM task_truth_v2_snapshots
                     WHERE snapshot_id=?1 AND packet_id=?2 LIMIT 1",
                    params![snapshot_id, packet_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|error| error.to_string())
            } else {
                // An unresolved attempt has no selected hypothesis, but it still
                // has one persisted boundary snapshot. Keep that exact packet
                // boundary instead of falling back to a different session row.
                conn.query_row(
                    "SELECT snapshot_json FROM task_truth_v2_snapshots
                     WHERE packet_id=?1 ORDER BY observed_at_ms DESC LIMIT 1",
                    params![packet_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|error| error.to_string())
            }
        })
        .transpose()?
        .flatten()
        .map(|raw| serde_json::from_str::<TaskSnapshotV2>(&raw).map_err(|error| error.to_string()))
        .transpose()?;
    if decision_id.is_some() && attempt_identity.is_none() {
        reason_codes.push("manual_attempt_audit_missing".into());
    }
    if attempt_packet_id.is_some() && attempt_snapshot.is_none() {
        reason_codes.push("manual_attempt_snapshot_missing".into());
    }
    let answer_snapshot = if decision_id.is_some() {
        // A manual Continue attempt is not allowed to borrow semantics or a
        // failure state from any older snapshot in the same session.
        attempt_snapshot.as_ref()
    } else {
        newest
            .filter(|snapshot| {
                snapshot.selection_status
                    == super::task_snapshot::SnapshotSelectionStatusV2::Unresolved
            })
            .or(selected)
    };
    let pftu_probe_result = if super::semantic_probe::public_authority_enabled() {
        decision_id
            .map(|decision_id| pftu_probe_public_result(conn, decision_id))
            .transpose()?
            .flatten()
    } else {
        None
    };
    if pftu_probe_result
        .as_ref()
        .and_then(|result| result.answer.as_ref())
        .is_some()
    {
        reason_codes.push("pftu_model_answer_routed_to_public_contract".into());
    }
    // A decision-bound manual Continue may only expose the compact provider
    // result. Background/read-only projection can still read an older verified
    // snapshot, but the legacy full-packet model output cannot replace a
    // missing or rejected compact answer.
    let mut answer = pftu_probe_result
        .as_ref()
        .and_then(|result| result.answer.clone())
        .or_else(|| {
            decision_id
                .is_none()
                .then(|| answer_snapshot.map(public_answer))
                .flatten()
        });
    if let Some(answer) = answer.as_mut() {
        apply_scoped_feedback(conn, answer)?;
        if let Some(reason) = enforce_model_first_semantic_authority(answer) {
            reason_codes.push(reason);
        }
        answer.recompute_product_projection();
    }
    let diagnostic_packet_id = attempt_packet_id.or_else(|| {
        answer_snapshot
            .or(newest)
            .or(selected)
            .map(|snapshot| snapshot.packet_id.as_str())
    });
    let inference_diagnostic = pftu_probe_result
        .as_ref()
        .map(|result| result.diagnostic.clone())
        .or(inference_diagnostic(
            conn,
            diagnostic_packet_id,
            decision_id,
        )?);
    if answer.is_none() && model_first_answer_required {
        reason_codes.push("typed_unresolved_model_first_answer".into());
        answer = Some(typed_unresolved_answer(
            session_id,
            inference_diagnostic.as_ref(),
            answer_snapshot,
            diagnostic_packet_id,
        ));
    }
    let release_report_fingerprint = release_report_identity(release_gate_source.as_deref());
    let fingerprint_material = serde_json::json!({
        "snapshot_schema": TASK_SNAPSHOT_SCHEMA_V2,
        "model_response_schema": TASK_TRUTH_MODEL_OUTPUT_SCHEMA_V1,
        "public_answer_schema": TASK_TRUTH_PUBLIC_ANSWER_SCHEMA_V1,
        "verifier_version": TASK_TRUTH_VERIFIER_VERSION,
        "selection_policy": SNAPSHOT_SELECTION_POLICY_V1,
        "authority_policy": TASK_TRUTH_AUTHORITY_POLICY_V1,
        "release_gate_passed": release_gate_passed,
        "release_gate_source": release_gate_source,
        "release_report_fingerprint": release_report_fingerprint,
        "requested_state": requested_state,
        "effective_state": effective_state,
        "provider_name": answer.as_ref().and_then(|answer| answer.provider_name.as_deref()).or_else(|| inference_diagnostic.as_ref().map(|diagnostic| diagnostic.provider.as_str())),
        "provider_model": answer.as_ref().and_then(|answer| answer.provider_model.as_deref()).or_else(|| inference_diagnostic.as_ref().map(|diagnostic| diagnostic.model.as_str())),
        "provider_request_id": answer.as_ref().and_then(|answer| answer.request_id.as_deref()).or_else(|| inference_diagnostic.as_ref().and_then(|diagnostic| diagnostic.request_id.as_deref())),
        "session_id": answer.as_ref().and_then(|answer| answer.atomic_identity.session_id.as_deref()),
        "task_thread_id": answer.as_ref().and_then(|answer| answer.atomic_identity.task_thread_id.as_deref()),
        "task_thread_revision": answer.as_ref().and_then(|answer| answer.atomic_identity.task_thread_revision),
        "snapshot_id": answer.as_ref().map(|answer| answer.atomic_identity.task_snapshot_id.as_str()),
        "snapshot_revision": answer.as_ref().map(|answer| answer.atomic_identity.snapshot_revision),
        "selected_hypothesis_id": answer.as_ref().and_then(|answer| answer.atomic_identity.selected_hypothesis_id.as_deref()),
        "model_response_id": answer.as_ref().and_then(|answer| answer.atomic_identity.model_response_id.as_deref()),
        "observation_packet_id": answer.as_ref().map(|answer| answer.atomic_identity.observation_packet_id.as_str()),
        "evidence_watermark": answer.as_ref().map(|answer| answer.atomic_identity.evidence_watermark.as_str()),
        "feedback_fingerprint": answer.as_ref().map(|answer| answer.atomic_identity.correction_fingerprint.as_str()),
        "resolver_version": answer_snapshot.map(|snapshot| snapshot.resolver_version.as_str()),
    });
    let decision = TaskTruthProductionDecisionV1 {
        schema: "smalltalk.task_truth_production_decision.v1".into(),
        requested_state,
        effective_state,
        policy_version: TASK_TRUTH_AUTHORITY_POLICY_V1.into(),
        release_gate_passed,
        release_gate_source,
        reason_codes,
        cache_fingerprint: stable_hash(fingerprint_material.to_string().as_bytes()),
        answer,
        inference_diagnostic,
    };
    audit_switch(conn, &decision)?;
    Ok(decision)
}

pub(crate) fn attach_observed_activity(
    decision: &mut TaskTruthProductionDecisionV1,
    work_truth: &ContinueWorkTruth,
) {
    if work_truth.resolution_status == "activity_supported" {
        decision
            .reason_codes
            .push("local_observed_activity_excluded_from_semantic_answer".into());
    }
}

pub(crate) fn attach_strict_target(
    decision: &mut TaskTruthProductionDecisionV1,
    snapshot_task_thread_id: Option<&str>,
    snapshot_task_thread_revision: Option<i64>,
    target_task_thread_id: Option<&str>,
    target_task_thread_revision: Option<i64>,
    direct_target_allowed: bool,
    target: Option<ContinueReturnTarget>,
) {
    // Target attachment is deliberately narrower than freshness. Callers only
    // enter this function for a real candidate target. Failing to prove that
    // candidate belongs to this semantic answer suppresses the target; it does
    // not make the freshly produced semantic answer old.
    debug_assert!(
        target.is_some(),
        "strict target attachment requires a candidate"
    );
    let identity_matches = snapshot_task_thread_id.is_some()
        && snapshot_task_thread_revision.is_some()
        && snapshot_task_thread_id == target_task_thread_id
        && snapshot_task_thread_revision == target_task_thread_revision;
    if let Some(answer) = decision.answer.as_mut() {
        let feedback_allows_target = answer.task_resolution_status != "unresolved"
            && answer
                .field_support
                .get("where_summary")
                .is_none_or(|support| support.support_status != "rejected_by_user");
        let target_identity = target
            .as_ref()
            .and_then(|target| serde_json::to_vec(target).ok())
            .map(|bytes| stable_hash(&bytes));
        if identity_matches && direct_target_allowed && feedback_allows_target && target.is_some() {
            answer.direct_return_target = target;
            answer.target_status = "direct_target_ready".into();
            answer.atomic_identity.target_identity = target_identity;
        } else {
            answer.direct_return_target = None;
            answer.atomic_identity.target_identity = None;
            if !identity_matches {
                answer.target_status = "target_suppressed".into();
                decision
                    .reason_codes
                    .push("target_task_identity_mismatch".into());
            } else if !feedback_allows_target {
                answer.target_status = "target_suppressed".into();
                decision
                    .reason_codes
                    .push("target_blocked_by_scoped_task_feedback".into());
            } else if target_identity.is_some() && !direct_target_allowed {
                answer.target_status = "target_suppressed".into();
            }
        }
        answer.recompute_product_projection();
    }
}

/// Proves that a local return target is the exact return anchor selected for
/// the persisted semantic snapshot. A legacy candidate, URL, or openable
/// artifact is not ownership evidence by itself.
pub(crate) fn strict_target_owner(
    conn: &Connection,
    answer: Option<&TaskTruthPublicAnswerV1>,
    target: &ContinueReturnTarget,
) -> Result<Option<(String, i64)>, String> {
    let Some(answer) = answer else {
        return Ok(None);
    };
    let identity = &answer.atomic_identity;
    let (Some(thread_id), Some(thread_revision), Some(hypothesis_id)) = (
        identity.task_thread_id.as_deref(),
        identity.task_thread_revision,
        identity.selected_hypothesis_id.as_deref(),
    ) else {
        return Ok(None);
    };
    if target.artifact_id.is_none() || target.openability != "openable" {
        return Ok(None);
    }

    checkpoint::ensure_schema(conn)?;
    let snapshot_raw = conn
        .query_row(
            "SELECT snapshot_json FROM task_truth_v2_snapshots
             WHERE snapshot_id=?1 AND revision=?2",
            params![identity.task_snapshot_id, identity.snapshot_revision],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some(snapshot_raw) = snapshot_raw else {
        return Ok(None);
    };
    let snapshot: TaskSnapshotV2 =
        serde_json::from_str(&snapshot_raw).map_err(|error| error.to_string())?;
    let snapshot_matches = snapshot.task_thread_id.as_deref() == Some(thread_id)
        && snapshot.task_thread_revision == Some(thread_revision)
        && snapshot.selected_hypothesis_id.as_deref() == Some(hypothesis_id)
        && snapshot.provider_response_id == identity.model_response_id
        && snapshot.packet_id == identity.observation_packet_id
        && snapshot.evidence_watermark == identity.evidence_watermark;
    if !snapshot_matches {
        return Ok(None);
    }
    let Some(anchor_id) = snapshot.return_anchor_candidate_id.as_deref() else {
        return Ok(None);
    };

    let packet_raw = conn
        .query_row(
            "SELECT packet_json FROM task_truth_v2_observation_packets WHERE packet_id=?1",
            params![identity.observation_packet_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some(packet_raw) = packet_raw else {
        return Ok(None);
    };
    let packet: ObservationPacketV2 =
        serde_json::from_str(&packet_raw).map_err(|error| error.to_string())?;
    let Some(anchor) = packet
        .return_anchor_facts
        .iter()
        .find(|anchor| anchor.record_id == anchor_id)
    else {
        return Ok(None);
    };
    if target.fallback_frame_id.as_deref() != anchor.frame_id.as_deref() {
        return Ok(None);
    }
    let locator = match anchor.source_kind.as_str() {
        "return_anchor_fact:browser_url" => target.browser_url.as_deref(),
        "return_anchor_fact:document_path" => target.document_path.as_deref(),
        _ => None,
    };
    let locator_hash = locator
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| stable_hash(value.as_bytes()));
    if locator_hash.as_deref() != anchor.content_hash.as_deref() {
        return Ok(None);
    }
    Ok(Some((thread_id.to_string(), thread_revision)))
}
pub(crate) fn persist_decision_contract(
    conn: &Connection,
    decision_id: &str,
    decision: &TaskTruthProductionDecisionV1,
) -> Result<(), String> {
    checkpoint::ensure_schema(conn)?;
    let answer = decision.answer.as_ref();
    let answer_contract_json = answer
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| error.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO task_truth_v2_decision_contracts (
           decision_id, effective_state, release_gate_passed, snapshot_id,
           snapshot_revision, task_thread_id, task_thread_revision,
           selected_hypothesis_id, model_request_id, model_response_id,
           provider_attempt_count, observation_packet_id, evidence_watermark,
           correction_fingerprint, current_frame_id, packet_policy_version,
           response_schema_version, admission_version, admitted_result_id,
           correction_watermark, target_status, target_identity,
           answer_contract_json, return_target_artifact_id, created_at_ms
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,
                   ?16,?17,?18,?19,?20,?21,?22,?23,?24,
                   CAST(strftime('%s','now') AS INTEGER) * 1000)",
        params![
            decision_id,
            decision.effective_state.label(),
            i64::from(decision.release_gate_passed),
            nonempty_identity(answer.map(|answer| answer.snapshot_id.as_str())),
            answer.and_then(|answer| {
                nonempty_identity(Some(answer.snapshot_id.as_str()))
                    .map(|_| answer.snapshot_revision)
            }),
            nonempty_identity(
                answer.and_then(|answer| answer.atomic_identity.task_thread_id.as_deref())
            ),
            answer.and_then(|answer| answer.atomic_identity.task_thread_revision),
            nonempty_identity(
                answer.and_then(|answer| answer.atomic_identity.selected_hypothesis_id.as_deref())
            ),
            nonempty_identity(
                answer.and_then(|answer| answer.atomic_identity.model_request_id.as_deref())
            ),
            nonempty_identity(
                answer.and_then(|answer| answer.atomic_identity.model_response_id.as_deref())
            ),
            decision
                .inference_diagnostic
                .as_ref()
                .map(|diagnostic| diagnostic.provider_attempt_count as i64)
                .unwrap_or_default(),
            nonempty_identity(
                answer.map(|answer| answer.atomic_identity.observation_packet_id.as_str())
            ),
            nonempty_identity(
                answer.map(|answer| answer.atomic_identity.evidence_watermark.as_str())
            ),
            nonempty_identity(
                answer.map(|answer| answer.atomic_identity.correction_fingerprint.as_str())
            ),
            nonempty_identity(
                answer.map(|answer| answer.atomic_identity.current_frame_id.as_str())
            ),
            nonempty_identity(
                answer.map(|answer| answer.atomic_identity.packet_policy_version.as_str())
            ),
            nonempty_identity(
                answer.map(|answer| answer.atomic_identity.response_schema_version.as_str())
            ),
            nonempty_identity(
                answer.map(|answer| answer.atomic_identity.admission_version.as_str())
            ),
            nonempty_identity(
                answer.map(|answer| answer.atomic_identity.admitted_result_id.as_str())
            ),
            nonempty_identity(
                answer.map(|answer| answer.atomic_identity.correction_watermark.as_str())
            ),
            nonempty_identity(answer.map(|answer| answer.target_status.as_str())),
            nonempty_identity(
                answer.and_then(|answer| answer.atomic_identity.target_identity.as_deref())
            ),
            answer_contract_json,
            answer
                .and_then(|answer| answer.direct_return_target.as_ref())
                .and_then(|target| target.artifact_id.as_deref()),
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn nonempty_identity(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuation::task_truth_v2::observation_packet::{
        ActiveSurfaceIdentityV2, EvidenceHandleV2, EvidencePartitionV2, KeyframeReferenceV2,
        ObservationPacketV2, PacketSizeAccountingV2,
    };
    use crate::continuation::task_truth_v2::task_snapshot::{
        unresolved_snapshot, SnapshotSelectionStatusV2,
    };

    fn product_projection_answer() -> TaskTruthPublicAnswerV1 {
        TaskTruthPublicAnswerV1 {
            atomic_answer_identity: "answer-identity".into(),
            admitted_semantic_status: "resolved".into(),
            task_resolution_status: "resolved".into(),
            unfinished_task: Some("Verify the Continue presentation".into()),
            task_state: "active".into(),
            resume_point: Some("The canonical backend answer is ready".into()),
            next_supported_action: Some("Inspect the shared Continue answer".into()),
            completed_context: Some("The backend mapping passed".into()),
            target_status: "task_known_target_unknown".into(),
            ..Default::default()
        }
    }

    #[test]
    fn product_projection_uses_only_supported_action_and_bounds_copy() {
        let mut answer = product_projection_answer();
        answer.next_supported_action = Some("a".repeat(200));
        answer.unfinished_state = Some("Open the unsafe compatibility target".into());
        answer.resume_point = Some("b".repeat(220));
        answer.recompute_product_projection();

        let projection = &answer.product_projection;
        assert_eq!(
            projection.presentation_state,
            ContinuePresentationStateV1::ActionKnown
        );
        assert_eq!(projection.primary_instruction.chars().count(), 160);
        assert!(projection.primary_instruction.ends_with('…'));
        assert!(!projection.primary_instruction.contains("unsafe"));
        assert_eq!(
            projection
                .resume_context
                .as_deref()
                .unwrap()
                .chars()
                .count(),
            180
        );
    }

    #[test]
    fn product_projection_has_precise_partial_and_failure_states() {
        let mut answer = product_projection_answer();
        answer.next_supported_action = None;
        answer.unfinished_state = Some("This must never become the instruction".into());
        answer.recompute_product_projection();
        assert_eq!(
            answer.product_projection.presentation_state,
            ContinuePresentationStateV1::TaskKnownActionUnknown
        );
        assert_eq!(
            answer.product_projection.primary_instruction,
            "I found the task, but not a safe next step."
        );

        answer.unfinished_task = None;
        answer.unresolved_or_failure_reason = Some("provider_timeout".into());
        answer.recompute_product_projection();
        assert_eq!(
            answer.product_projection.presentation_state,
            ContinuePresentationStateV1::ProviderFailure
        );
        assert_eq!(answer.stale_product_projection, answer.product_projection);

        answer.unfinished_task = Some("Known task must not erase the provider failure".into());
        answer.next_supported_action = Some("Unsafe action must not win".into());
        answer.recompute_product_projection();
        assert_eq!(
            answer.product_projection.presentation_state,
            ContinuePresentationStateV1::ProviderFailure
        );
        assert_eq!(answer.stale_product_projection, answer.product_projection);
        answer.unfinished_task = None;
        answer.next_supported_action = None;

        answer.unresolved_or_failure_reason = Some("invalid_json_parse".into());
        answer.recompute_product_projection();
        assert_eq!(
            answer.product_projection.presentation_state,
            ContinuePresentationStateV1::ParserFailure
        );
        assert_eq!(answer.stale_product_projection, answer.product_projection);

        answer.unresolved_or_failure_reason = Some("support_validation_failure".into());
        answer.recompute_product_projection();
        assert_eq!(
            answer.product_projection.presentation_state,
            ContinuePresentationStateV1::ValidationFailure
        );
        assert_eq!(answer.stale_product_projection, answer.product_projection);

        answer.unresolved_or_failure_reason =
            Some("manual_continue_boundary_capture_failed".into());
        answer.recompute_product_projection();
        assert_eq!(
            answer.product_projection.presentation_state,
            ContinuePresentationStateV1::CaptureFailure
        );
        assert_eq!(answer.stale_product_projection, answer.product_projection);

        answer.unresolved_or_failure_reason = Some("semantic_unresolved".into());
        answer.recompute_product_projection();
        assert_eq!(
            answer.product_projection.presentation_state,
            ContinuePresentationStateV1::TaskUnknown
        );
        assert_eq!(
            answer.stale_product_projection, answer.product_projection,
            "task acquisition failure must not become stale copy after evidence advances"
        );
        assert_ne!(
            answer.stale_product_projection.primary_instruction,
            "The saved answer is older than the latest work."
        );
    }

    #[test]
    fn product_projection_maps_direct_preview_and_stale_actions_safely() {
        let mut answer = product_projection_answer();
        answer.target_status = "direct_target_ready".into();
        answer.direct_return_target = Some(ContinueReturnTarget {
            artifact_id: Some("artifact-owned".into()),
            artifact_kind: Some("browser_tab".into()),
            title: Some("Owned target".into()),
            browser_url: Some("https://example.invalid/owned".into()),
            document_path: None,
            openability: "openable".into(),
            fallback_frame_id: Some("frame-owned".into()),
        });
        answer.recompute_product_projection();
        assert_eq!(
            answer.product_projection.primary_action.kind,
            ContinueProductActionKindV1::OpenDirectTarget
        );
        assert_eq!(
            answer.stale_product_projection.primary_action.kind,
            ContinueProductActionKindV1::RefreshContinue
        );
        assert_eq!(
            answer.stale_product_projection.answer_identity,
            answer.product_projection.answer_identity
        );
        assert!(answer.stale_product_projection.location_context.is_none());

        answer.direct_return_target = None;
        answer.target_status = "frame_preview_only".into();
        answer.evidence_preview = Some(ContinueEvidencePreview {
            schema: "smalltalk.continue_evidence_preview.v1".into(),
            preview_kind: "answer_evidence".into(),
            frame_id: "frame-owned".into(),
        });
        answer.recompute_product_projection();
        assert_eq!(
            answer.product_projection.primary_action.kind,
            ContinueProductActionKindV1::InspectEvidence
        );
        assert_eq!(
            answer.product_projection.primary_action.label,
            "View last screen"
        );

        answer.target_status = "stale_decision".into();
        answer.recompute_product_projection();
        assert_eq!(
            answer.product_projection.presentation_state,
            ContinuePresentationStateV1::StaleDecision
        );
        assert_eq!(
            answer.product_projection.primary_action.kind,
            ContinueProductActionKindV1::RefreshContinue
        );
    }

    #[test]
    fn lca_06_fresh_target_failures_preserve_semantics_and_true_stale_refreshes() {
        let mut unresolved = product_projection_answer();
        unresolved.unfinished_task = None;
        unresolved.next_supported_action = None;
        unresolved.target_status = "no_task".into();
        unresolved.recompute_product_projection();
        assert_eq!(
            unresolved.product_projection.presentation_state,
            ContinuePresentationStateV1::TaskUnknown
        );

        let mut answer = product_projection_answer();
        answer.target_status = "frame_preview_only".into();
        answer.evidence_preview = Some(ContinueEvidencePreview {
            schema: "smalltalk.continue_evidence_preview.v1".into(),
            preview_kind: "answer_evidence".into(),
            frame_id: "frame-current".into(),
        });
        answer.recompute_product_projection();
        let instruction = answer.product_projection.primary_instruction.clone();
        let mut decision = TaskTruthProductionDecisionV1 {
            answer: Some(answer),
            ..Default::default()
        };
        attach_strict_target(
            &mut decision,
            Some("thread-current"),
            Some(2),
            Some("thread-other"),
            Some(1),
            true,
            Some(ContinueReturnTarget {
                artifact_id: Some("artifact-other".into()),
                artifact_kind: Some("browser_tab".into()),
                title: Some("Unowned candidate".into()),
                browser_url: Some("https://example.invalid/unowned".into()),
                document_path: None,
                openability: "openable".into(),
                fallback_frame_id: None,
            }),
        );
        let answer = decision.answer.as_ref().unwrap();
        assert_eq!(answer.target_status, "target_suppressed");
        assert_eq!(answer.product_projection.primary_instruction, instruction);
        assert_ne!(
            answer.product_projection.presentation_state,
            ContinuePresentationStateV1::StaleDecision
        );
        assert!(answer.direct_return_target.is_none());

        let mut stale = answer.clone();
        stale.target_status = "stale_decision".into();
        stale.unresolved_or_failure_reason = Some("material_evidence_watermark_advanced".into());
        stale.recompute_product_projection();
        assert_eq!(
            stale.product_projection.presentation_state,
            ContinuePresentationStateV1::StaleDecision
        );
        assert_eq!(
            stale.product_projection.primary_action.kind,
            ContinueProductActionKindV1::RefreshContinue
        );
    }

    #[test]
    fn product_projection_defaults_when_old_answer_is_deserialized() {
        let mut value = serde_json::to_value(product_projection_answer()).unwrap();
        value.as_object_mut().unwrap().remove("product_projection");
        let restored: TaskTruthPublicAnswerV1 = serde_json::from_value(value).unwrap();
        assert_eq!(
            restored.product_projection,
            ContinueProductProjectionV1::default()
        );
        assert_eq!(
            restored.stale_product_projection.presentation_state,
            ContinuePresentationStateV1::StaleDecision
        );
    }

    fn arm_probe_case_for_run_fk(conn: &Connection, case_id: &str) {
        super::super::semantic_probe::arm_case(
            conn,
            &super::super::semantic_probe::ArmedProbeCase {
                case_id: case_id.into(),
                case_kind: "production_mapping_fixture".into(),
                held_back: false,
                expected_recorded_at_ms: 1,
                expected_primary_task: None,
                expected_current_step: None,
                expected_last_progress: None,
                expected_unfinished_state: None,
                recoverable_by_field: BTreeMap::from([
                    ("primary_task".into(), false),
                    ("current_step".into(), false),
                    ("last_progress".into(), false),
                    ("unfinished_state".into(), false),
                ]),
            },
        )
        .unwrap();
    }

    fn feedback_packet() -> ObservationPacketV2 {
        let observed_at_ms = 1_000;
        let current_frame = KeyframeReferenceV2 {
            frame_id: "frame-feedback".into(),
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
            task_evidence_role: None,
            task_turn_id: None,
            same_task_relation: "unknown".into(),
            cross_pane_ambiguity: false,
            near_duplicate_group: None,
        };
        ObservationPacketV2 {
            schema: "smalltalk.observation_packet.v2".into(),
            packet_id: "packet-feedback".into(),
            observed_at_ms,
            session_id: Some("session-feedback".into()),
            evidence_watermark: "watermark-feedback".into(),
            active_surface: current_frame.surface_identity.clone(),
            current_frame: current_frame.clone(),
            semantic_keyframes: vec![current_frame],
            surface_timeline: Vec::new(),
            task_relevance: Default::default(),
            image_candidates: Vec::new(),
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
                vec!["frame-feedback".into()],
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

    fn persist_thread_owned_feedback_snapshot(conn: &Connection) {
        let packet = feedback_packet();
        let mut snapshot = unresolved_snapshot(&packet, None, "feedback_fixture");
        snapshot.snapshot_id = "snapshot-exact".into();
        snapshot.revision = 7;
        snapshot.task_thread_id = Some("thread-exact".into());
        snapshot.task_thread_revision = Some(7);
        snapshot.thread_status = "active".into();
        snapshot.task_basis = "explicit_goal".into();
        snapshot.task_summary = Some("Implement thread-scoped feedback".into());
        snapshot.task_kind = "model_inferred".into();
        snapshot.execution_state = "active".into();
        snapshot.relation_to_prior = "continuation".into();
        snapshot.selection_status = SnapshotSelectionStatusV2::Selected;
        snapshot.semantic_source = "cloud_multimodal_model".into();
        snapshot.selected_hypothesis_id = Some("hypothesis-selected".into());
        snapshot.provider_response_id = Some("response-feedback".into());
        checkpoint::persist_checkpoint(conn, &packet, &snapshot).unwrap();
    }

    fn persist_real_feedback_thread(
        conn: &Connection,
    ) -> super::super::task_thread::ThreadBoundaryPersistResultV1 {
        let packet = feedback_packet();
        let mut snapshot = unresolved_snapshot(&packet, None, "feedback_thread_fixture");
        snapshot.snapshot_id = "snapshot-thread-feedback".into();
        snapshot.task_basis = "explicit_goal".into();
        snapshot.task_summary = Some("Implement thread-scoped feedback".into());
        snapshot.task_kind = "model_inferred".into();
        snapshot.execution_state = "active".into();
        snapshot.relation_to_prior = "new_task".into();
        snapshot.selection_status = SnapshotSelectionStatusV2::Selected;
        snapshot.semantic_source = "cloud_multimodal_model".into();
        snapshot.selected_hypothesis_id = Some("hypothesis-selected".into());
        snapshot.provider_response_id = Some("response-feedback".into());
        super::super::task_thread::persist_boundary_atomic(conn, &packet, snapshot).unwrap()
    }

    #[test]
    fn manual_production_decision_never_promotes_legacy_full_packet_semantics() {
        let conn = Connection::open_in_memory().unwrap();
        checkpoint::ensure_schema(&conn).unwrap();

        let packet = feedback_packet();
        let mut snapshot = unresolved_snapshot(&packet, None, "manual_attempt_fixture");
        snapshot.snapshot_id = "snapshot-attempt".into();
        snapshot.revision = 3;
        snapshot.task_thread_id = Some("thread-attempt".into());
        snapshot.task_thread_revision = Some(3);
        snapshot.thread_status = "active".into();
        snapshot.task_basis = "model_inferred".into();
        snapshot.task_summary = Some("Repair the exact Continue attempt".into());
        snapshot.task_object = Some("Continue inference flow".into());
        snapshot.execution_state = "active".into();
        snapshot.relation_to_prior = "continuation".into();
        snapshot.selection_status = SnapshotSelectionStatusV2::Selected;
        snapshot.semantic_source = "cloud_multimodal_model".into();
        snapshot.selected_hypothesis_id = Some("hypothesis-attempt".into());
        snapshot.provider_name = Some("openai".into());
        snapshot.provider_model = Some("gpt-test".into());
        snapshot.provider_request_id = Some("request-attempt".into());
        snapshot.provider_response_id = Some("response-attempt".into());
        checkpoint::persist_checkpoint(&conn, &packet, &snapshot).unwrap();

        // A later row can share the packet after a correction or migration.
        // The decision audit still names snapshot-attempt, so production must
        // not select this higher revision merely because it is newer.
        let mut same_packet_newer =
            unresolved_snapshot(&packet, Some(&snapshot), "same_packet_newer");
        same_packet_newer.snapshot_id = "snapshot-same-packet-newer".into();
        same_packet_newer.revision = 99;
        checkpoint::persist_checkpoint(&conn, &packet, &same_packet_newer).unwrap();

        let mut newer_packet = packet.clone();
        newer_packet.packet_id = "packet-newer-unresolved".into();
        newer_packet.observed_at_ms = 2_000;
        newer_packet.evidence_watermark = "watermark-newer".into();
        let mut newer = unresolved_snapshot(&newer_packet, Some(&snapshot), "newer_unresolved");
        newer.snapshot_id = "snapshot-newer-unresolved".into();
        checkpoint::persist_checkpoint(&conn, &newer_packet, &newer).unwrap();

        let packet_summary = serde_json::json!({
            "multimodal": {
                "resolver": {
                    "diagnostic_status": "success",
                    "origin": "live_cloud",
                    "provider": "openai",
                    "model": "gpt-test",
                    "request_id": "request-attempt",
                    "provider_request_id": "provider-request-attempt",
                    "response_id": "response-attempt",
                    "latency_ms": 10,
                    "usage": {"input_tokens": 100, "output_tokens": 20, "total_tokens": 120},
                    "request_audit": {"image_count": 1, "image_bytes": 100, "estimated_tokens": 25}
                },
                "verification": {
                    "status": "resolved",
                    "snapshot": {"selected_hypothesis_id": "hypothesis-attempt"}
                }
            }
        });
        conn.execute(
            "INSERT INTO task_truth_v2_shadow_audits (
               audit_id, decision_id, observed_at_ms, packet_id, selected_snapshot_id,
               legacy_task_turn_id, first_divergence, packet_summary_json,
               keyframe_reasons_json, canonical_conflicts_json, causal_edges_json,
               snapshot_hypotheses_json, selection_json, legacy_comparison_json,
               latency_ms, serialized_bytes, estimated_tokens, created_at_ms
             ) VALUES (?1,?2,?3,?4,?5,NULL,NULL,?6,'[]','[]','[]','[]','{}','{}',10,100,25,?3)",
            params![
                "audit-attempt",
                "decision-attempt",
                packet.observed_at_ms,
                packet.packet_id,
                snapshot.snapshot_id,
                packet_summary.to_string(),
            ],
        )
        .unwrap();

        let decision = production_decision_for_attempt(
            &conn,
            Some("session-feedback"),
            true,
            Some("decision-attempt"),
        )
        .unwrap();
        persist_decision_contract(&conn, "decision-attempt", &decision).unwrap();
        let answer = decision.answer.as_ref().unwrap();
        assert_eq!(answer.task_resolution_status, "unresolved");
        assert!(answer.task_summary.is_none());
        assert_eq!(
            answer.atomic_identity.observation_packet_id,
            "packet-feedback"
        );
        assert_eq!(answer.atomic_identity.task_snapshot_id, "snapshot-attempt");
        assert_eq!(answer.atomic_identity.snapshot_revision, 3);
        assert_eq!(
            answer.atomic_identity.evidence_watermark,
            packet.evidence_watermark
        );
        assert_eq!(
            answer.atomic_identity.model_request_id.as_deref(),
            Some("request-attempt")
        );
        assert_eq!(
            answer.atomic_identity.model_response_id.as_deref(),
            Some("response-attempt")
        );
        assert!(decision
            .reason_codes
            .iter()
            .any(|reason| reason == "typed_unresolved_model_first_answer"));
        let diagnostic = decision.inference_diagnostic.as_ref().unwrap();
        assert_eq!(diagnostic.provider, "openai");
        assert_eq!(diagnostic.response_id.as_deref(), Some("response-attempt"));
        let persisted_identity = conn
            .query_row(
                "SELECT observation_packet_id, snapshot_id, snapshot_revision,
                        model_request_id, model_response_id, answer_contract_json
                 FROM task_truth_v2_decision_contracts WHERE decision_id=?1",
                params!["decision-attempt"],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(persisted_identity.0.as_deref(), Some("packet-feedback"));
        assert_eq!(persisted_identity.1.as_deref(), Some("snapshot-attempt"));
        assert_eq!(persisted_identity.2, Some(3));
        assert_eq!(persisted_identity.3.as_deref(), Some("request-attempt"));
        assert_eq!(persisted_identity.4.as_deref(), Some("response-attempt"));
        let persisted_answer: TaskTruthPublicAnswerV1 =
            serde_json::from_str(persisted_identity.5.as_deref().unwrap()).unwrap();
        assert_eq!(&persisted_answer, answer);
    }

    #[test]
    fn authoritative_request_cannot_bypass_closed_release_gate() {
        let requested = TaskTruthAuthorityStateV1::Authoritative;
        let release_gate_passed = false;
        let effective =
            if requested == TaskTruthAuthorityStateV1::Authoritative && !release_gate_passed {
                TaskTruthAuthorityStateV1::Eligible
            } else {
                requested
            };
        assert_eq!(effective, TaskTruthAuthorityStateV1::Eligible);
    }

    #[test]
    fn local_semantic_source_is_forced_to_typed_unresolved() {
        let mut answer = TaskTruthPublicAnswerV1 {
            task_resolution_status: "resolved".into(),
            task_summary: Some("Browsing a visible page title".into()),
            task_object: Some("Visible page".into()),
            next_action: Some("Keep browsing".into()),
            semantic_source: "local_scorer".into(),
            task_understanding_source: "local_causal".into(),
            atomic_identity: TaskTruthAtomicIdentityV1 {
                task_thread_id: Some("thread-local".into()),
                task_thread_revision: Some(1),
                task_snapshot_id: "snapshot-local".into(),
                selected_hypothesis_id: Some("hypothesis-local".into()),
                observation_packet_id: "packet-local".into(),
                evidence_watermark: "watermark-local".into(),
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            enforce_model_first_semantic_authority(&mut answer).as_deref(),
            Some("unsupported_semantic_source")
        );
        assert_eq!(answer.task_resolution_status, "unresolved");
        assert_eq!(answer.semantic_source, "unresolved");
        assert!(answer.task_summary.is_none());
        assert!(answer.task_object.is_none());
        assert!(answer.next_action.is_none());
    }

    #[test]
    fn cloud_semantics_without_response_identity_are_forced_unresolved() {
        let mut answer = TaskTruthPublicAnswerV1 {
            task_resolution_status: "resolved".into(),
            task_summary: Some("Implement model-first authority".into()),
            semantic_source: "cloud_multimodal_model".into(),
            selected_hypothesis_id: Some("hypothesis-cloud".into()),
            atomic_identity: TaskTruthAtomicIdentityV1 {
                task_thread_id: Some("thread-cloud".into()),
                task_thread_revision: Some(2),
                task_snapshot_id: "snapshot-cloud".into(),
                selected_hypothesis_id: Some("hypothesis-cloud".into()),
                observation_packet_id: "packet-cloud".into(),
                evidence_watermark: "watermark-cloud".into(),
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            enforce_model_first_semantic_authority(&mut answer).as_deref(),
            Some("missing_model_response_identity")
        );
        assert_eq!(answer.task_resolution_status, "unresolved");
        assert!(answer.task_summary.is_none());
    }

    #[test]
    fn manual_model_first_request_without_snapshot_gets_typed_unresolved_answer() {
        let conn = Connection::open_in_memory().unwrap();
        checkpoint::ensure_schema(&conn).unwrap();

        let decision = production_decision(&conn, Some("session-empty"), true).unwrap();
        let answer = decision.answer.expect("manual request must have an answer");

        assert_eq!(answer.task_resolution_status, "unresolved");
        assert_eq!(answer.semantic_source, "unresolved");
        assert_eq!(answer.inference_status, "no_verified_snapshot");
        assert_eq!(
            answer.atomic_identity.session_id.as_deref(),
            Some("session-empty")
        );
        assert!(answer.task_summary.is_none());
        assert!(decision
            .reason_codes
            .iter()
            .any(|reason| reason == "typed_unresolved_model_first_answer"));
    }

    #[test]
    fn passed_boolean_without_complete_release_evidence_cannot_open_authority() {
        let forged = serde_json::json!({
            "schema": "smalltalk.mfti_04.final_release_report.v1",
            "policy_version": "mfti.04-v1",
            "passed": true,
            "authority_state": "authoritative",
            "violations": []
        });
        assert!(!final_release_report_is_complete(&forged));
    }

    #[test]
    fn historical_tt2_report_cannot_open_model_first_authority() {
        let historical = serde_json::json!({
            "schema": "smalltalk.task_truth_v2.final_release_report.v1",
            "policy_version": "tt2.02-v1",
            "passed": true,
            "authority_state": "authoritative",
            "violations": []
        });
        assert!(!final_release_report_is_complete(&historical));
    }

    #[test]
    fn complete_release_report_shape_can_open_authority() {
        let metric_names = [
            "wrong_primary_task_rate",
            "visible_surface_substituted_for_task",
            "wrong_activity_to_task_relationship",
            "wrong_task_switch_or_detour",
            "cross_session_stale_leakage",
            "mixed_snapshot_semantic_fields",
            "control_navigation_as_task_rate",
            "unsupported_specific_claim_rate",
            "provider_failure_local_semantic_fallback",
            "provider_failure_honest_unresolved",
            "useful_non_generic_task_summary",
            "task_object_accuracy",
            "execution_state_accuracy",
            "supported_next_action_precision",
            "supported_next_action_coverage",
            "return_target_precision",
            "stronger_manual_result_downgraded",
            "unseen_application_useful_summary",
            "human_immediately_useful",
        ];
        let surfaces = [
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
        let metrics = metric_names
            .iter()
            .map(|name| {
                (
                    (*name).to_string(),
                    serde_json::json!({"passed": true, "denominator": 200}),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        let mut intervals = metric_names
            .iter()
            .map(|name| {
                (
                    (*name).to_string(),
                    serde_json::json!({"method": "wilson_score", "lower": 0.9, "upper": 1.0}),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        intervals.extend(surfaces.iter().map(|name| {
            (
                format!("wrong_primary_task_rate.surface.{name}"),
                serde_json::json!({"method": "wilson_score", "lower": 0.0, "upper": 0.1}),
            )
        }));
        let surface_results = surfaces
            .iter()
            .map(|name| {
                (
                    (*name).to_string(),
                    serde_json::json!({"passed": true, "denominator": 20}),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        let report = serde_json::json!({
            "schema": "smalltalk.mfti_04.final_release_report.v1",
            "policy_version": "mfti.04-v1",
            "passed": true,
            "authority_state": "authoritative",
            "violations": [],
            "reviewed_live_count": 200,
            "locked_holdout_count": 50,
            "manual_scenario_count": 10,
            "performance_sample_count": 30,
            "metric_results": metrics,
            "confidence_intervals": intervals,
            "surface_results": surface_results,
            "bindings": {
                "corpus_sha256": "sha256:corpus", "holdout_sha256": "sha256:holdout",
                "provider": "openai", "model": "test-model", "prompt_version": "v1",
                "response_schema_version": "v2", "observation_packet_version": "v2",
                "verifier_version": "v1", "task_thread_version": "v1",
                "public_answer_version": "v2", "performance_privacy_policy_version": "v1",
                "manual_qa_manifest_sha256": "sha256:manual", "source_commit": "commit",
                "build_identity": "build"
            },
            "validation": {
                "policy": [], "evaluator": [], "release_identity": [],
                "manual_macos_qa": [], "performance_cost_privacy": []
            }
        });
        assert!(final_release_report_is_complete(&report));
    }

    #[test]
    fn target_mismatch_nulls_target_without_rewriting_task() {
        let mut decision = TaskTruthProductionDecisionV1::default();
        decision.answer = Some(TaskTruthPublicAnswerV1 {
            schema: TASK_TRUTH_PUBLIC_ANSWER_SCHEMA_V1.into(),
            task_basis: "explicit_goal".into(),
            task_resolution_status: "resolved".into(),
            task_summary: Some("Implement Task Truth authority".into()),
            task_object: None,
            last_meaningful_progress: None,
            unfinished_state: None,
            next_action: None,
            where_summary: None,
            alternative_hypotheses: Vec::new(),
            direct_return_target: None,
            evidence_preview: None,
            field_support: BTreeMap::new(),
            task_understanding_source: "local_causal".into(),
            wording_source: "deterministic".into(),
            target_selection_source: "strict_local_policy".into(),
            snapshot_id: "snapshot-1".into(),
            snapshot_revision: 1,
            evidence_watermark: "watermark-1".into(),
            ..TaskTruthPublicAnswerV1::default()
        });
        attach_strict_target(
            &mut decision,
            Some("turn-current"),
            Some(1),
            Some("turn-old"),
            Some(1),
            true,
            Some(ContinueReturnTarget {
                artifact_id: None,
                artifact_kind: None,
                title: Some("Old tab".into()),
                browser_url: Some("https://example.invalid".into()),
                document_path: None,
                openability: "openable".into(),
                fallback_frame_id: None,
            }),
        );
        let answer = decision.answer.unwrap();
        assert_eq!(
            answer.task_summary.as_deref(),
            Some("Implement Task Truth authority")
        );
        assert!(answer.direct_return_target.is_none());
    }

    #[test]
    fn persisted_return_anchor_proves_exact_target_owner_and_rejects_locator_mismatch() {
        let conn = Connection::open_in_memory().unwrap();
        let url = "https://example.invalid/owned";
        let mut packet = feedback_packet();
        packet.return_anchor_facts = vec![EvidenceHandleV2 {
            source_kind: "return_anchor_fact:browser_url".into(),
            record_id: "frame-feedback:browser_url".into(),
            frame_id: Some("frame-feedback".into()),
            content_hash: Some(stable_hash(url.as_bytes())),
        }];
        let mut snapshot = unresolved_snapshot(&packet, None, "target_owner_fixture");
        snapshot.snapshot_id = "snapshot-target-owner".into();
        snapshot.task_basis = "explicit_goal".into();
        snapshot.task_summary = Some("Open the owned target".into());
        snapshot.task_kind = "model_inferred".into();
        snapshot.execution_state = "active".into();
        snapshot.relation_to_prior = "new_task".into();
        snapshot.selection_status = SnapshotSelectionStatusV2::Selected;
        snapshot.semantic_source = "cloud_multimodal_model".into();
        snapshot.selected_hypothesis_id = Some("hypothesis-target-owner".into());
        snapshot.provider_response_id = Some("response-target-owner".into());
        snapshot.return_anchor_candidate_id = Some("frame-feedback:browser_url".into());
        let persisted =
            super::super::task_thread::persist_boundary_atomic(&conn, &packet, snapshot).unwrap();
        let answer = public_answer(&persisted.snapshot);
        let target = ContinueReturnTarget {
            artifact_id: Some("artifact-owned".into()),
            artifact_kind: Some("browser_tab".into()),
            title: Some("Owned".into()),
            browser_url: Some(url.into()),
            document_path: None,
            openability: "openable".into(),
            fallback_frame_id: Some("frame-feedback".into()),
        };

        assert_eq!(
            strict_target_owner(&conn, Some(&answer), &target).unwrap(),
            persisted
                .selected_thread_id
                .zip(persisted.selected_thread_revision)
        );
        let mut mismatched = target;
        mismatched.browser_url = Some("https://example.invalid/other".into());
        assert!(strict_target_owner(&conn, Some(&answer), &mismatched)
            .unwrap()
            .is_none());
    }

    #[test]
    fn matching_thread_id_with_different_revision_cannot_attach_target() {
        let mut decision = TaskTruthProductionDecisionV1::default();
        decision.answer = Some(TaskTruthPublicAnswerV1 {
            task_resolution_status: "resolved".into(),
            task_summary: Some("Exact revision task".into()),
            ..Default::default()
        });
        attach_strict_target(
            &mut decision,
            Some("thread-a"),
            Some(2),
            Some("thread-a"),
            Some(1),
            true,
            Some(ContinueReturnTarget {
                artifact_id: Some("artifact-a".into()),
                artifact_kind: None,
                title: None,
                browser_url: Some("https://example.invalid".into()),
                document_path: None,
                openability: "openable".into(),
                fallback_frame_id: None,
            }),
        );
        assert!(decision.answer.unwrap().direct_return_target.is_none());
        assert!(decision
            .reason_codes
            .contains(&"target_task_identity_mismatch".into()));
    }

    #[test]
    fn observed_activity_cannot_create_or_mutate_a_semantic_answer() {
        let mut decision = TaskTruthProductionDecisionV1::default();
        let truth = ContinueWorkTruth {
            schema: super::super::super::CONTINUE_WORK_TRUTH_SCHEMA.into(),
            policy_version: super::super::super::CONTINUE_WORK_TRUTH_POLICY_VERSION.into(),
            resolution_status: "activity_supported".into(),
            activity_kind: "editing".into(),
            activity_summary: Some("Editing tt2-05-completion-audit.md".into()),
            work_object: Some("tt2-05-completion-audit.md".into()),
            where_summary: Some("Visual Studio Code".into()),
            app_name: Some("Visual Studio Code".into()),
            artifact_id: Some("artifact-md".into()),
            observed_at_ms: 1_000,
            confidence: 0.88,
            evidence_ids: vec!["frame-1".into()],
            source: "local_direct_activity".into(),
            broader_goal_known: false,
            primary_relation: "primary".into(),
            reason_codes: vec!["direct_production_action".into()],
        };

        attach_observed_activity(&mut decision, &truth);

        assert!(decision.answer.is_none());
        assert!(decision
            .reason_codes
            .iter()
            .any(|reason| reason == "local_observed_activity_excluded_from_semantic_answer"));
    }

    #[test]
    fn matching_strict_target_attaches_without_rewriting_semantic_fields() {
        let mut decision = TaskTruthProductionDecisionV1::default();
        decision.answer = Some(TaskTruthPublicAnswerV1 {
            task_resolution_status: "resolved".into(),
            task_summary: Some("Implement Task Truth authority".into()),
            where_summary: Some("Codex".into()),
            snapshot_id: "snapshot-1".into(),
            snapshot_revision: 1,
            evidence_watermark: "watermark-1".into(),
            ..Default::default()
        });
        attach_strict_target(
            &mut decision,
            Some("turn-current"),
            Some(1),
            Some("turn-current"),
            Some(1),
            true,
            Some(ContinueReturnTarget {
                artifact_id: Some("artifact-current".into()),
                artifact_kind: Some("code_file".into()),
                title: Some("production.rs".into()),
                browser_url: None,
                document_path: Some("/private/redacted/production.rs".into()),
                openability: "openable".into(),
                fallback_frame_id: None,
            }),
        );
        let answer = decision.answer.unwrap();
        assert_eq!(
            answer.task_summary.as_deref(),
            Some("Implement Task Truth authority")
        );
        assert_eq!(answer.where_summary.as_deref(), Some("Codex"));
        assert_eq!(
            answer
                .direct_return_target
                .and_then(|target| target.artifact_id),
            Some("artifact-current".into())
        );
    }

    #[test]
    fn not_right_feedback_is_scoped_to_exact_snapshot_revision_and_field() {
        let conn = Connection::open_in_memory().unwrap();
        let initial = persist_real_feedback_thread(&conn);
        let original = initial.snapshot;
        let result = crate::continuation::record_continue_feedback(
            &conn,
            crate::continuation::ContinueExplicitFeedbackRequest {
                decision_id: None,
                selected_candidate_id: None,
                workstream_id: None,
                target_artifact_id: None,
                corrected_artifact_id: None,
                feedback_kind: "rejected".into(),
                note: None,
                source: Some("test".into()),
                task_snapshot_id: Some(original.snapshot_id.clone()),
                task_snapshot_revision: Some(original.revision),
                affected_task_field: Some("task_summary".into()),
                task_hypothesis_id: None,
            },
        )
        .unwrap();
        assert!(result.workstream_id.is_none());
        assert!(result.target_artifact_id.is_none());
        assert!(result.normalized_targets.is_empty());
        let stored: (String, i64, String, String, String, i64, i64) = conn
            .query_row(
                "SELECT task_snapshot_id, task_snapshot_revision, affected_field, feedback_kind,
                        corrected_snapshot_id, corrected_snapshot_revision, task_thread_revision
                 FROM task_truth_v2_feedback_events WHERE feedback_id=?1",
                params![result.id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(stored.0.as_str(), original.snapshot_id.as_str());
        assert_eq!(stored.1, original.revision);
        assert_eq!(stored.2, "task_summary");
        assert_eq!(stored.3, "rejected");
        assert_eq!(stored.5, original.revision + 1);
        assert_eq!(stored.6, 2);
        let corrected_raw: String = conn
            .query_row(
                "SELECT snapshot_json FROM task_truth_v2_snapshots
                 WHERE snapshot_id=?1 AND revision=?2",
                params![stored.4, stored.5],
                |row| row.get(0),
            )
            .unwrap();
        let corrected: TaskSnapshotV2 = serde_json::from_str(&corrected_raw).unwrap();
        assert_eq!(corrected.thread_status, "unresolved");
        assert_eq!(corrected.semantic_source, "human_correction");
        assert!(corrected.task_summary.is_none());
        assert!(corrected.last_meaningful_progress.is_none());
        assert!(corrected.unfinished_step.is_none());
        assert!(corrected.next_action.is_none());
        let public = production_decision(&conn, Some("session-feedback"), false)
            .unwrap()
            .answer
            .unwrap();
        assert_eq!(public.task_resolution_status, "unresolved");
        assert_eq!(
            public.atomic_identity.task_snapshot_id,
            corrected.snapshot_id
        );
        let legacy_feedback_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM continue_feedback_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(legacy_feedback_count, 0);
        let first_watermark =
            crate::continuation::current_continue_evidence_watermark_hash(&conn, None).unwrap();
        let second = crate::continuation::record_continue_feedback(
            &conn,
            crate::continuation::ContinueExplicitFeedbackRequest {
                decision_id: None,
                selected_candidate_id: None,
                workstream_id: None,
                target_artifact_id: None,
                corrected_artifact_id: None,
                feedback_kind: "rejected".into(),
                note: None,
                source: Some("test".into()),
                task_snapshot_id: Some(original.snapshot_id.clone()),
                task_snapshot_revision: Some(original.revision),
                affected_task_field: Some("next_action".into()),
                task_hypothesis_id: None,
            },
        )
        .unwrap();
        let second_watermark =
            crate::continuation::current_continue_evidence_watermark_hash(&conn, None).unwrap();
        assert_ne!(result.id, second.id);
        assert_ne!(first_watermark, second_watermark);
        let scoped_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_truth_v2_feedback_events",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(scoped_count, 2);
    }

    #[test]
    fn relationship_correction_creates_a_thread_revision_without_global_suppression() {
        let conn = Connection::open_in_memory().unwrap();
        let initial = persist_real_feedback_thread(&conn);
        let original = initial.snapshot;
        let thread_id = original.task_thread_id.clone().unwrap();

        crate::continuation::record_continue_feedback(
            &conn,
            crate::continuation::ContinueExplicitFeedbackRequest {
                decision_id: Some("decision-relationship".into()),
                selected_candidate_id: None,
                workstream_id: None,
                target_artifact_id: None,
                corrected_artifact_id: None,
                feedback_kind: "supporting_work".into(),
                note: None,
                source: Some("test".into()),
                task_snapshot_id: Some(original.snapshot_id.clone()),
                task_snapshot_revision: Some(original.revision),
                affected_task_field: Some("relationship".into()),
                task_hypothesis_id: None,
            },
        )
        .unwrap();

        let corrected_raw: String = conn
            .query_row(
                "SELECT snapshot_json FROM task_truth_v2_snapshots
                 WHERE snapshot_id<>?1
                 ORDER BY observed_at_ms DESC LIMIT 1",
                params![original.snapshot_id],
                |row| row.get(0),
            )
            .unwrap();
        let corrected: TaskSnapshotV2 = serde_json::from_str(&corrected_raw).unwrap();
        assert_eq!(
            corrected.task_thread_id.as_deref(),
            Some(thread_id.as_str())
        );
        assert_eq!(corrected.task_thread_revision, Some(2));
        assert_eq!(corrected.relation_to_prior, "supporting_research");
        assert_eq!(corrected.semantic_source, "human_correction");
        assert_eq!(corrected.app_identity.as_deref(), Some("Test App"));
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM continue_feedback_events", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
            0
        );
    }

    #[test]
    fn cache_identity_changes_for_every_atomic_semantic_input() {
        let conn = Connection::open_in_memory().unwrap();
        persist_thread_owned_feedback_snapshot(&conn);
        let decide = || {
            production_decision(&conn, Some("session-feedback"), false)
                .unwrap()
                .cache_fingerprint
        };
        let baseline = decide();
        let other_session = production_decision(&conn, Some("session-other"), false)
            .unwrap()
            .cache_fingerprint;
        assert_ne!(baseline, other_session);

        let snapshot_raw: String = conn
            .query_row(
                "SELECT snapshot_json FROM task_truth_v2_snapshots
                 WHERE snapshot_id='snapshot-exact' AND revision=7",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let mut snapshot: TaskSnapshotV2 = serde_json::from_str(&snapshot_raw).unwrap();

        snapshot.task_thread_revision = Some(8);
        conn.execute(
            "UPDATE task_truth_v2_snapshots SET snapshot_json=?1
             WHERE snapshot_id='snapshot-exact' AND revision=7",
            params![serde_json::to_string(&snapshot).unwrap()],
        )
        .unwrap();
        let thread_revision_changed = decide();
        assert_ne!(baseline, thread_revision_changed);

        snapshot.provider_response_id = Some("response-feedback-2".into());
        conn.execute(
            "UPDATE task_truth_v2_snapshots SET snapshot_json=?1
             WHERE snapshot_id='snapshot-exact' AND revision=7",
            params![serde_json::to_string(&snapshot).unwrap()],
        )
        .unwrap();
        let response_changed = decide();
        assert_ne!(thread_revision_changed, response_changed);

        snapshot.evidence_watermark = "watermark-feedback-2".into();
        conn.execute(
            "UPDATE task_truth_v2_snapshots SET snapshot_json=?1
             WHERE snapshot_id='snapshot-exact' AND revision=7",
            params![serde_json::to_string(&snapshot).unwrap()],
        )
        .unwrap();
        let watermark_changed = decide();
        assert_ne!(response_changed, watermark_changed);

        conn.execute(
            "INSERT INTO task_truth_v2_feedback_events (
               feedback_id, task_thread_id, task_thread_revision,
               task_snapshot_id, task_snapshot_revision, affected_field,
               evidence_watermark, feedback_kind, observed_at_ms
             ) VALUES ('feedback-cache','thread-exact',8,'snapshot-exact',7,
                       'next_action','watermark-feedback-2','rejected',2000)",
            [],
        )
        .unwrap();
        let corrected = production_decision(&conn, Some("session-feedback"), false).unwrap();
        assert_ne!(watermark_changed, corrected.cache_fingerprint);
        assert!(!corrected
            .answer
            .unwrap()
            .atomic_identity
            .correction_fingerprint
            .is_empty());
    }

    #[test]
    fn scoped_hypothesis_feedback_promotes_only_the_selected_alternative() {
        let conn = Connection::open_in_memory().unwrap();
        checkpoint::ensure_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO task_truth_v2_feedback_events (
               feedback_id, task_snapshot_id, task_snapshot_revision, affected_field,
               hypothesis_id, feedback_kind, decision_id, observed_at_ms
             ) VALUES ('feedback-a','snapshot-exact',7,'hypothesis','hypothesis-a',
                       'corrected','decision-a',100)",
            [],
        )
        .unwrap();
        let mut answer = TaskTruthPublicAnswerV1 {
            task_resolution_status: "ambiguous".into(),
            task_summary: Some("Original task".into()),
            alternative_hypotheses: vec![
                TaskTruthAlternativeV1 {
                    hypothesis_id: "hypothesis-a".into(),
                    task_summary: "User-selected task".into(),
                    relation: "alternative".into(),
                    confidence: 0.72,
                    evidence_refs: vec!["frame:1".into()],
                    ..Default::default()
                },
                TaskTruthAlternativeV1 {
                    hypothesis_id: "hypothesis-b".into(),
                    task_summary: "Other task".into(),
                    relation: "alternative".into(),
                    confidence: 0.7,
                    evidence_refs: vec!["frame:2".into()],
                    ..Default::default()
                },
            ],
            snapshot_id: "snapshot-exact".into(),
            snapshot_revision: 7,
            evidence_watermark: "watermark-7".into(),
            selected_hypothesis_id: Some("hypothesis-original".into()),
            atomic_identity: TaskTruthAtomicIdentityV1 {
                selected_hypothesis_id: Some("hypothesis-original".into()),
                ..Default::default()
            },
            ..Default::default()
        };

        apply_scoped_feedback(&conn, &mut answer).unwrap();

        assert_eq!(answer.task_summary.as_deref(), Some("User-selected task"));
        assert_eq!(answer.task_resolution_status, "resolved");
        assert_eq!(answer.task_understanding_source, "human_correction");
        assert_eq!(answer.semantic_source, "human_correction");
        assert_eq!(
            answer.selected_hypothesis_id.as_deref(),
            Some("hypothesis-a")
        );
        assert_eq!(
            answer.atomic_identity.selected_hypothesis_id.as_deref(),
            Some("hypothesis-a")
        );
        assert_eq!(answer.alternative_hypotheses.len(), 2);
        assert!(answer
            .alternative_hypotheses
            .iter()
            .any(|hypothesis| hypothesis.hypothesis_id == "hypothesis-b"));
        let demoted = answer
            .alternative_hypotheses
            .iter()
            .find(|hypothesis| hypothesis.hypothesis_id == "hypothesis-original")
            .expect("previously selected hypothesis remains inspectable");
        assert_eq!(demoted.disposition, "demoted_by_human_choice");
    }

    #[test]
    fn rejected_task_summary_blocks_target_for_that_snapshot_revision() {
        let conn = Connection::open_in_memory().unwrap();
        checkpoint::ensure_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO task_truth_v2_feedback_events (
               feedback_id, task_snapshot_id, task_snapshot_revision, affected_field,
               hypothesis_id, feedback_kind, decision_id, observed_at_ms
             ) VALUES ('feedback-task','snapshot-exact',7,'task_summary',NULL,
                       'rejected','decision-a',100)",
            [],
        )
        .unwrap();
        let mut decision = TaskTruthProductionDecisionV1::default();
        let mut answer = TaskTruthPublicAnswerV1 {
            task_resolution_status: "resolved".into(),
            task_summary: Some("Wrong task".into()),
            where_summary: Some("Codex".into()),
            snapshot_id: "snapshot-exact".into(),
            snapshot_revision: 7,
            evidence_watermark: "watermark-7".into(),
            ..Default::default()
        };
        apply_scoped_feedback(&conn, &mut answer).unwrap();
        decision.answer = Some(answer);
        attach_strict_target(
            &mut decision,
            Some("turn-current"),
            Some(7),
            Some("turn-current"),
            Some(7),
            true,
            Some(ContinueReturnTarget {
                artifact_id: Some("artifact-current".into()),
                artifact_kind: None,
                title: Some("Current target".into()),
                browser_url: Some("https://example.invalid/current".into()),
                document_path: None,
                openability: "openable".into(),
                fallback_frame_id: None,
            }),
        );
        let answer = decision.answer.unwrap();
        assert_eq!(answer.task_resolution_status, "unresolved");
        assert!(answer.task_summary.is_none());
        assert!(answer.direct_return_target.is_none());
        assert!(decision
            .reason_codes
            .iter()
            .any(|reason| reason == "target_blocked_by_scoped_task_feedback"));
    }

    #[test]
    fn unscoped_request_abstains_instead_of_selecting_any_snapshot() {
        let conn = Connection::open_in_memory().unwrap();
        checkpoint::ensure_schema(&conn).unwrap();
        let decision = production_decision(&conn, None, false).unwrap();
        assert!(decision.answer.is_none());
        assert!(decision
            .reason_codes
            .iter()
            .any(|reason| reason == "unscoped_request_no_snapshot_selection"));
    }

    #[test]
    fn parsed_pftu_model_answer_is_public_even_without_an_open_target() {
        let conn = Connection::open_in_memory().unwrap();
        checkpoint::ensure_schema(&conn).unwrap();
        super::super::semantic_probe::ensure_schema(&conn).unwrap();
        arm_probe_case_for_run_fk(&conn, "case-visible");
        let output = super::super::semantic_probe::ProbeModelOutput {
            unfinished_task: Some("Fix Continue so GPT answers stay visible".into()),
            task_state: super::super::semantic_probe::ProbeTaskState::Active,
            resume_point: Some("Routing the PFTU response into the public answer".into()),
            next_supported_action: Some("Inspect the routed answer in the main app".into()),
            completed_context: Some("The paid provider response was parsed".into()),
            where_summary: Some("The Codex implementation result".into()),
            visit_roles: BTreeMap::from([
                (
                    "T1_VISIT".into(),
                    super::super::semantic_probe::ProbeVisitRole {
                        role: super::super::semantic_probe::ProbeSurfaceRole::PrimaryWork,
                        confidence: 0.94,
                        support_slots: vec!["T1_CONTEXT_IMAGE".into()],
                        relationship_to_primary_task: "This screen contains the primary work."
                            .into(),
                    },
                ),
                (
                    "T3_VISIT".into(),
                    super::super::semantic_probe::ProbeVisitRole {
                        role: super::super::semantic_probe::ProbeSurfaceRole::DetourOrUnrelated,
                        confidence: 0.81,
                        support_slots: vec!["B1_IMAGE_AFTER".into()],
                        relationship_to_primary_task:
                            "This is the current detour from the primary work.".into(),
                    },
                ),
            ]),
            support_slots_by_field: BTreeMap::from([
                ("unfinished_task".into(), Vec::new()),
                ("task_state".into(), Vec::new()),
                ("resume_point".into(), Vec::new()),
                ("next_supported_action".into(), Vec::new()),
                ("completed_context".into(), Vec::new()),
                ("where_summary".into(), Vec::new()),
            ]),
            missing_evidence: vec!["no_direct_return_locator".into()],
            missing_evidence_by_field: BTreeMap::from([(
                "where_summary".into(),
                vec!["exact locator not established".into()],
            )]),
            confidence_by_field: BTreeMap::from([
                ("unfinished_task".into(), 0.91),
                ("task_state".into(), 0.90),
                ("resume_point".into(), 0.88),
                ("next_supported_action".into(), 0.87),
                ("completed_context".into(), 0.86),
                ("where_summary".into(), 0.84),
            ]),
            verifier_result_by_field: [
                "unfinished_task",
                "task_state",
                "resume_point",
                "next_supported_action",
                "completed_context",
                "where_summary",
            ]
            .into_iter()
            .map(|field| {
                (
                    field.into(),
                    super::super::semantic_probe::ProbeFieldVerifierResult::Admitted,
                )
            })
            .collect(),
            status: super::super::semantic_probe::ProbeResolutionStatus::Resolved,
        };
        let request_audit = serde_json::json!({
            "request_schema": "smalltalk.pftu_01.semantic_probe_request.v3",
            "request_id": "request-local",
            "model": "gpt-test",
            "boundary_count": 1,
            "image_count": 4,
            "image_bytes": 100,
            "structured_bytes": 1000,
            "estimated_text_tokens": 250,
            "max_text_bytes": 24576,
            "max_estimated_text_tokens": 6144,
            "output_contract_field_count": 6,
            "supplied_image_slots": ["T1_CONTEXT_IMAGE", "B1_IMAGE_AFTER"],
            "missing_evidence": [],
            "surface_timeline": [
                {
                    "visit_id": "T1_VISIT",
                    "sequence_index": 1,
                    "app_label": "ChatGPT",
                    "site_hostname": null,
                    "first_observed_at_ms": 100,
                    "last_observed_at_ms": 200,
                    "is_current": false,
                    "revisited": false,
                    "private": false,
                    "image_slot": "T1_CONTEXT_IMAGE",
                    "evidence_refs": ["frame-489"]
                },
                {
                    "visit_id": "T2_VISIT",
                    "sequence_index": 2,
                    "app_label": "Private activity",
                    "site_hostname": null,
                    "first_observed_at_ms": 300,
                    "last_observed_at_ms": 400,
                    "is_current": false,
                    "revisited": false,
                    "private": true,
                    "image_slot": null,
                    "evidence_refs": ["frame-private"]
                },
                {
                    "visit_id": "T3_VISIT",
                    "sequence_index": 3,
                    "app_label": "Helium",
                    "site_hostname": "platform.openai.com",
                    "first_observed_at_ms": 500,
                    "last_observed_at_ms": 600,
                    "is_current": true,
                    "revisited": false,
                    "private": false,
                    "image_slot": "B1_IMAGE_AFTER",
                    "evidence_refs": ["frame-499"]
                }
            ]
        });
        conn.execute(
            "INSERT INTO task_truth_v2_semantic_probe_runs (
               run_id, case_id, decision_id, session_id, packet_id,
               evidence_watermark, model, diagnostic_status, request_id,
               provider_request_id, response_id, response_model,
               request_audit_json, cited_support_slots_json, admitted_output_json,
               validation_issues_json, latency_ms, parsed_response,
               provider_post_count, created_at_ms
             ) VALUES (
               'run-visible','case-visible','decision-visible','session-visible',
               'packet-visible','watermark-visible','gpt-test','success',
               'request-local','request-provider','response-provider','gpt-test',
               ?1,'{}',?2,'[]',321,1,1,1000
             )",
            [
                serde_json::to_string(&request_audit).unwrap(),
                serde_json::to_string(&output).unwrap(),
            ],
        )
        .unwrap();

        let result = pftu_probe_public_result(&conn, "decision-visible")
            .unwrap()
            .expect("the exact decision-bound probe result should be found");
        let mut answer = result.answer.expect("parsed model text must become public");
        assert_eq!(answer.task_resolution_status, "resolved");
        assert_eq!(answer.model_resolution_status, "resolved");
        assert_eq!(answer.task_state, "active");
        assert_eq!(
            answer.task_summary.as_deref(),
            Some("Fix Continue so GPT answers stay visible")
        );
        assert_eq!(
            answer.current_subtask.as_deref(),
            Some("Routing the PFTU response into the public answer")
        );
        assert_eq!(answer.unfinished_task, answer.task_summary);
        assert_eq!(answer.resume_point, answer.current_subtask);
        assert_eq!(answer.next_supported_action, answer.next_action);
        assert_eq!(answer.completed_context, answer.last_meaningful_progress);
        assert_eq!(answer.task_basis, "verified_cloud_inferred_goal");
        assert!(answer.direct_return_target.is_none());
        assert_eq!(answer.response_id.as_deref(), Some("response-provider"));
        assert_eq!(answer.recent_context.len(), 3);
        assert_eq!(answer.schema, "smalltalk.task_truth_public_answer.v6");
        assert_eq!(answer.recent_context[0].app_label, "ChatGPT");
        assert_eq!(
            answer.recent_context[0].semantic_role.as_deref(),
            Some("primary_work")
        );
        assert_eq!(answer.recent_context[1].app_label, "Private activity");
        assert!(answer.recent_context[1].site_hostname.is_none());
        assert_eq!(
            answer.recent_context[2].site_hostname.as_deref(),
            Some("platform.openai.com")
        );
        assert_eq!(
            answer.recent_context[2].semantic_role.as_deref(),
            Some("detour_or_unrelated")
        );
        assert_eq!(
            answer.current_activity.relationship_to_primary,
            "detour_or_unrelated"
        );
        assert_eq!(result.diagnostic.status, "success");
        assert_eq!(result.diagnostic.provider_attempt_count, 1);
        assert_eq!(enforce_model_first_semantic_authority(&mut answer), None);
        assert_eq!(
            answer.task_summary.as_deref(),
            Some("Fix Continue so GPT answers stay visible"),
            "target safety and provenance validation must not erase the model answer"
        );

        let decision = production_decision_for_attempt(
            &conn,
            Some("session-visible"),
            true,
            Some("decision-visible"),
        )
        .expect("project exact compact result through production");
        let answer = decision.answer.expect("compact result must be public");
        assert_eq!(
            answer.task_summary.as_deref(),
            Some("Fix Continue so GPT answers stay visible")
        );
        assert_eq!(answer.response_id.as_deref(), Some("response-provider"));
        assert!(decision
            .reason_codes
            .iter()
            .any(|reason| reason == "pftu_model_answer_routed_to_public_contract"));

        // A non-primary unsupported field remains field-local. Keep the task
        // and the other admitted fields visible.
        let mut field_limited_output = output.clone();
        field_limited_output.resume_point = None;
        field_limited_output.verifier_result_by_field.insert(
            "resume_point".into(),
            super::super::semantic_probe::ProbeFieldVerifierResult::Rejected,
        );
        field_limited_output.status =
            super::super::semantic_probe::ProbeResolutionStatus::PartlyResolved;
        conn.execute(
            "UPDATE task_truth_v2_semantic_probe_runs
             SET diagnostic_status='support_slot_validation_failure',
                 admitted_output_json=?2,
                 validation_issues_json='[\"resume_point:unsupported\"]'
             WHERE decision_id=?1",
            params![
                "decision-visible",
                serde_json::to_string(&field_limited_output).unwrap()
            ],
        )
        .unwrap();
        let field_limited = pftu_probe_public_result(&conn, "decision-visible")
            .unwrap()
            .expect("field-limited result")
            .answer
            .expect("supported fields must remain public");
        assert_eq!(field_limited.task_resolution_status, "partly_resolved");
        assert_eq!(
            field_limited.task_summary.as_deref(),
            Some("Fix Continue so GPT answers stay visible")
        );
        assert!(field_limited.current_subtask.is_none());
        assert_eq!(
            field_limited.last_meaningful_progress.as_deref(),
            Some("The paid provider response was parsed")
        );
        assert!(field_limited.unfinished_state.is_none());
        assert_eq!(
            field_limited.next_action.as_deref(),
            Some("Inspect the routed answer in the main app")
        );
        assert_eq!(
            field_limited.inference_status,
            "model_answer_visible_with_validation_limits"
        );
        let field_limited_decision = production_decision_for_attempt(
            &conn,
            Some("session-visible"),
            true,
            Some("decision-visible"),
        )
        .expect("project field-limited output through the production decision");
        let field_limited_public = field_limited_decision
            .answer
            .expect("production must preserve the supported fields");
        assert_eq!(
            field_limited_public.task_resolution_status,
            "partly_resolved"
        );
        assert_eq!(
            field_limited_public.task_summary.as_deref(),
            Some("Fix Continue so GPT answers stay visible")
        );
        assert!(field_limited_public.current_subtask.is_none());
        assert_eq!(
            field_limited_public.last_meaningful_progress.as_deref(),
            Some("The paid provider response was parsed")
        );
        assert!(field_limited_public.unfinished_state.is_none());
        assert_eq!(field_limited_public.task_state, "active");

        // A rejected primary-task claim must stay rejected, but it must not
        // erase other fields that independently passed local admission. The
        // product can show those fields as a limited, inspect-only answer while
        // still refusing to claim a broad task or open an exact target.
        let mut primary_rejected_output = output.clone();
        primary_rejected_output.unfinished_task = None;
        primary_rejected_output.verifier_result_by_field.insert(
            "unfinished_task".into(),
            super::super::semantic_probe::ProbeFieldVerifierResult::Rejected,
        );
        primary_rejected_output.visit_roles.clear();
        primary_rejected_output.status =
            super::super::semantic_probe::ProbeResolutionStatus::Unresolved;
        conn.execute(
            "UPDATE task_truth_v2_semantic_probe_runs
             SET diagnostic_status='support_slot_validation_failure',
                 admitted_output_json=?2,
                 validation_issues_json='[\"unfinished_task:passive_evidence_cannot_establish_primary_task\"]'
             WHERE decision_id=?1",
            params![
                "decision-visible",
                serde_json::to_string(&primary_rejected_output).unwrap()
            ],
        )
        .unwrap();
        let primary_rejected = pftu_probe_public_result(&conn, "decision-visible")
            .unwrap()
            .expect("rejected task remains diagnostically inspectable")
            .answer
            .expect("factual timeline remains public");
        assert_eq!(primary_rejected.task_resolution_status, "unresolved");
        assert!(primary_rejected.task_summary.is_none());
        assert_eq!(
            primary_rejected.current_subtask.as_deref(),
            Some("Routing the PFTU response into the public answer")
        );
        assert_eq!(
            primary_rejected.last_meaningful_progress.as_deref(),
            Some("The paid provider response was parsed")
        );
        assert_eq!(
            primary_rejected.unfinished_state.as_deref(),
            Some("Routing the PFTU response into the public answer")
        );
        assert!(primary_rejected
            .recent_context
            .iter()
            .all(|visit| visit.semantic_role.is_none()));

        // Maintenance invalidation is categorically different from a
        // field-local rejection. A stale admitted JSON value must not leak
        // after the packet identity has been invalidated.
        conn.execute(
            "UPDATE task_truth_v2_semantic_probe_runs
             SET diagnostic_status='invalidated_identity_conflict'
             WHERE decision_id=?1",
            ["decision-visible"],
        )
        .unwrap();
        let invalidated = pftu_probe_public_result(&conn, "decision-visible")
            .unwrap()
            .expect("invalidated diagnostic remains inspectable");
        let invalidated_answer = invalidated
            .answer
            .expect("factual timeline remains inspectable");
        assert_eq!(
            invalidated.diagnostic.status,
            "invalidated_identity_conflict"
        );
        assert_eq!(invalidated_answer.task_resolution_status, "unresolved");
        assert!(invalidated_answer.current_subtask.is_none());
        assert!(invalidated_answer.last_meaningful_progress.is_none());
        assert!(invalidated_answer.unfinished_state.is_none());
    }

    #[test]
    fn factual_where_summary_alone_never_creates_openability_or_legacy_task_fill() {
        let output = super::super::semantic_probe::ProbeModelOutput {
            unfinished_task: None,
            task_state: super::super::semantic_probe::ProbeTaskState::Unclear,
            resume_point: None,
            next_supported_action: None,
            completed_context: None,
            where_summary: Some("The visible answer section in Codex".into()),
            visit_roles: BTreeMap::new(),
            support_slots_by_field: [
                "unfinished_task",
                "task_state",
                "resume_point",
                "next_supported_action",
                "completed_context",
                "where_summary",
            ]
            .into_iter()
            .map(|field| (field.into(), Vec::new()))
            .collect(),
            missing_evidence: vec!["exact return locator unavailable".into()],
            missing_evidence_by_field: BTreeMap::from([(
                "where_summary".into(),
                vec!["location is descriptive only".into()],
            )]),
            confidence_by_field: BTreeMap::from([
                ("unfinished_task".into(), 0.0),
                ("task_state".into(), 0.0),
                ("resume_point".into(), 0.0),
                ("next_supported_action".into(), 0.0),
                ("completed_context".into(), 0.0),
                ("where_summary".into(), 0.72),
            ]),
            verifier_result_by_field: BTreeMap::from([
                (
                    "unfinished_task".into(),
                    super::super::semantic_probe::ProbeFieldVerifierResult::NotProposed,
                ),
                (
                    "task_state".into(),
                    super::super::semantic_probe::ProbeFieldVerifierResult::NotProposed,
                ),
                (
                    "resume_point".into(),
                    super::super::semantic_probe::ProbeFieldVerifierResult::NotProposed,
                ),
                (
                    "next_supported_action".into(),
                    super::super::semantic_probe::ProbeFieldVerifierResult::NotProposed,
                ),
                (
                    "completed_context".into(),
                    super::super::semantic_probe::ProbeFieldVerifierResult::NotProposed,
                ),
                (
                    "where_summary".into(),
                    super::super::semantic_probe::ProbeFieldVerifierResult::Admitted,
                ),
            ]),
            status: super::super::semantic_probe::ProbeResolutionStatus::PartlyResolved,
        };
        let answer = map_compact_probe_output_to_public_answer(
            &output,
            &BTreeMap::new(),
            &CompactProbePublicMappingContext {
                decision_id: "decision-where-only".into(),
                session_id: Some("session-where-only".into()),
                packet_id: "packet-where-only".into(),
                evidence_watermark: "watermark-where-only".into(),
                configured_model: "gpt-test".into(),
                response_model: None,
                request_id: Some("request-where-only".into()),
                provider_request_id: None,
                provider_response_id: Some("response-where-only".into()),
                diagnostic_status: "support_slot_validation_failure".into(),
                validation_issues: Vec::new(),
                recent_context: Vec::new(),
                current_frame_id: "frame-where-only".into(),
                packet_policy_version: "test-packet-policy".into(),
                response_schema_version: super::super::semantic_probe::PROBE_RESPONSE_SCHEMA.into(),
                explicit_goal_support_slots: Vec::new(),
                correction_watermark: String::new(),
                semantic_conflicts: Vec::new(),
            },
        );

        assert_eq!(
            answer.where_summary.as_deref(),
            Some("The visible answer section in Codex")
        );
        assert!(answer.direct_return_target.is_none());
        assert!(answer.unfinished_task.is_none());
        assert!(answer.task_summary.is_none());
        assert!(answer.current_subtask.is_none());
        assert!(answer.unfinished_state.is_none());
        assert!(answer.next_action.is_none());
    }

    #[test]
    fn missing_provider_native_response_id_does_not_erase_parsed_model_text() {
        let mut answer = TaskTruthPublicAnswerV1 {
            task_resolution_status: "resolved".into(),
            task_summary: Some("Keep the parsed GPT answer visible".into()),
            semantic_source: "cloud_multimodal_model".into(),
            response_id: None,
            atomic_identity: TaskTruthAtomicIdentityV1 {
                task_thread_id: Some("thread-visible".into()),
                task_thread_revision: Some(1),
                task_snapshot_id: "snapshot-visible".into(),
                snapshot_revision: 1,
                selected_hypothesis_id: Some("hypothesis-visible".into()),
                model_response_id: Some("provider-response-envelope-local".into()),
                observation_packet_id: "packet-visible".into(),
                evidence_watermark: "watermark-visible".into(),
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(enforce_model_first_semantic_authority(&mut answer), None);
        assert_eq!(
            answer.task_summary.as_deref(),
            Some("Keep the parsed GPT answer visible")
        );
    }

    #[test]
    fn old_public_answer_rows_deserialize_without_recent_context() {
        let mut legacy = serde_json::to_value(TaskTruthPublicAnswerV1::default()).unwrap();
        legacy["schema"] = serde_json::json!("smalltalk.task_truth_public_answer.v2");
        let legacy_object = legacy.as_object_mut().expect("public answer object");
        for field in [
            "recent_context",
            "model_resolution_status",
            "unfinished_task",
            "task_state",
            "resume_point",
            "next_supported_action",
            "completed_context",
        ] {
            legacy_object.remove(field);
        }

        let restored: TaskTruthPublicAnswerV1 = serde_json::from_value(legacy).unwrap();
        assert_eq!(restored.schema, "smalltalk.task_truth_public_answer.v2");
        assert!(restored.recent_context.is_empty());
        assert_eq!(restored.model_resolution_status, "unresolved");
        assert!(restored.unfinished_task.is_none());
        assert_eq!(restored.task_state, "unclear");
        assert!(restored.resume_point.is_none());
        assert!(restored.next_supported_action.is_none());
        assert!(restored.completed_context.is_none());

        let mut v3 = serde_json::to_value(TaskTruthPublicAnswerV1::default()).unwrap();
        v3["schema"] = serde_json::json!("smalltalk.task_truth_public_answer.v3");
        v3["recent_context"] = serde_json::json!([{
            "sequence_index": 1,
            "app_label": "Helium",
            "site_hostname": "developers.openai.com",
            "first_observed_at_ms": 100,
            "last_observed_at_ms": 200,
            "is_current": true,
            "revisited": false,
            "evidence_refs": ["frame-1"]
        }]);
        let restored_v3: TaskTruthPublicAnswerV1 = serde_json::from_value(v3).unwrap();
        assert_eq!(restored_v3.recent_context.len(), 1);
        assert!(restored_v3.recent_context[0].semantic_role.is_none());
        assert!(restored_v3.recent_context[0].role_evidence_refs.is_empty());
    }

    #[test]
    fn stored_pre_lca_compact_row_is_explicitly_downgraded_without_semantic_fill() {
        let conn = Connection::open_in_memory().unwrap();
        super::super::semantic_probe::ensure_schema(&conn).unwrap();
        arm_probe_case_for_run_fk(&conn, "case-legacy");
        let old_compact_output = serde_json::json!({
            "primary_task": "Old broad project wording",
            "current_step": "Reviewing",
            "last_progress": "Implementation was completed",
            "unfinished_state": "Testing remains",
            "visit_roles": {},
            "support_slots_by_field": {
                "primary_task": [],
                "current_step": [],
                "last_progress": [],
                "unfinished_state": []
            },
            "missing_evidence": [],
            "confidence_by_field": {
                "primary_task": 0.9,
                "current_step": 0.8,
                "last_progress": 0.8,
                "unfinished_state": 0.8
            },
            "status": "resolved"
        });
        conn.execute(
            "INSERT INTO task_truth_v2_semantic_probe_runs (
               run_id, case_id, decision_id, session_id, packet_id,
               evidence_watermark, model, diagnostic_status, request_id,
               response_id, admitted_output_json, validation_issues_json,
               latency_ms, parsed_response, provider_post_count, created_at_ms
             ) VALUES (
               'run-legacy','case-legacy','decision-legacy','session-legacy',
               'packet-legacy','watermark-legacy','gpt-test','success','request-legacy',
               'response-legacy',?1,'[]',10,1,1,1000
             )",
            [serde_json::to_string(&old_compact_output).unwrap()],
        )
        .unwrap();

        let result = pftu_probe_public_result(&conn, "decision-legacy")
            .unwrap()
            .expect("legacy diagnostic row remains inspectable");
        let answer = result.answer.expect("typed unresolved downgrade");
        assert_eq!(
            result.diagnostic.status,
            "legacy_compact_contract_downgraded"
        );
        assert_eq!(answer.task_resolution_status, "unresolved");
        assert_eq!(
            answer.inference_status,
            "legacy_compact_contract_downgraded"
        );
        assert!(answer.unfinished_task.is_none());
        assert_eq!(answer.task_state, "unclear");
        assert!(answer.resume_point.is_none());
        assert!(answer.next_supported_action.is_none());
        assert!(answer.completed_context.is_none());
        assert!(answer.task_summary.is_none());
        assert!(answer.current_subtask.is_none());
        assert!(answer.last_meaningful_progress.is_none());
        assert!(answer.unfinished_state.is_none());
        assert!(answer.next_action.is_none());
        assert!(answer.where_summary.is_none());
        assert!(answer.direct_return_target.is_none());
    }

    #[test]
    fn unresolved_probe_still_projects_factual_recent_context() {
        let conn = Connection::open_in_memory().unwrap();
        super::super::semantic_probe::ensure_schema(&conn).unwrap();
        arm_probe_case_for_run_fk(&conn, "case-unresolved");
        let request_audit = serde_json::json!({
            "request_schema": "smalltalk.pftu_01.semantic_probe_request.v2",
            "request_id": "request-unresolved",
            "model": "gpt-test",
            "boundary_count": 1,
            "image_count": 1,
            "image_bytes": 10,
            "structured_bytes": 100,
            "estimated_text_tokens": 25,
            "max_text_bytes": 24576,
            "max_estimated_text_tokens": 6144,
            "output_contract_field_count": 4,
            "supplied_image_slots": ["B1_IMAGE_AFTER"],
            "missing_evidence": [],
            "surface_timeline": [{
                "sequence_index": 1,
                "app_label": "Helium",
                "site_hostname": "platform.openai.com",
                "first_observed_at_ms": 500,
                "last_observed_at_ms": 600,
                "is_current": true,
                "revisited": false,
                "private": false,
                "image_slot": "B1_IMAGE_AFTER",
                "evidence_refs": ["frame-499"]
            }]
        });
        conn.execute(
            "INSERT INTO task_truth_v2_semantic_probe_runs (
               run_id, case_id, decision_id, session_id, packet_id,
               evidence_watermark, model, diagnostic_status, request_id,
               request_audit_json, cited_support_slots_json,
               validation_issues_json, latency_ms, parsed_response,
               provider_post_count, created_at_ms
             ) VALUES (
               'run-unresolved','case-unresolved','decision-unresolved','session-unresolved',
               'packet-unresolved','watermark-unresolved','gpt-test','provider_no_usable_output',
               'request-unresolved',?1,'{}','[]',50,0,1,1000
             )",
            [serde_json::to_string(&request_audit).unwrap()],
        )
        .unwrap();

        let result = pftu_probe_public_result(&conn, "decision-unresolved")
            .unwrap()
            .expect("probe result");
        let answer = result.answer.expect("factual context answer");
        assert_eq!(result.diagnostic.status, "provider_no_usable_output");
        assert_eq!(answer.task_resolution_status, "unresolved");
        assert!(answer.task_summary.is_none());
        assert_eq!(answer.recent_context.len(), 1);
        assert_eq!(
            answer.recent_context[0].site_hostname.as_deref(),
            Some("platform.openai.com")
        );
    }

    fn complete_compact_output(
        status: super::super::semantic_probe::ProbeResolutionStatus,
    ) -> super::super::semantic_probe::ProbeModelOutput {
        use super::super::semantic_probe::{
            ProbeFieldVerifierResult, ProbeModelOutput, ProbeTaskState,
        };
        let fields = [
            "unfinished_task",
            "task_state",
            "resume_point",
            "next_supported_action",
            "completed_context",
            "where_summary",
        ];
        ProbeModelOutput {
            unfinished_task: Some("Verify the completed visual cue".into()),
            task_state: ProbeTaskState::NeedsUserVerification,
            resume_point: Some("Implementation is complete and verification remains".into()),
            next_supported_action: Some("Test the visual cue".into()),
            completed_context: Some("Implementation and checks completed".into()),
            where_summary: Some("The visual-cue task".into()),
            visit_roles: BTreeMap::new(),
            support_slots_by_field: fields
                .into_iter()
                .map(|field| (field.into(), vec![format!("{field}_slot")]))
                .collect(),
            missing_evidence: Vec::new(),
            missing_evidence_by_field: fields
                .into_iter()
                .map(|field| (field.into(), Vec::new()))
                .collect(),
            confidence_by_field: fields
                .into_iter()
                .map(|field| (field.into(), 0.9))
                .collect(),
            verifier_result_by_field: fields
                .into_iter()
                .map(|field| (field.into(), ProbeFieldVerifierResult::Admitted))
                .collect(),
            status,
        }
    }

    #[test]
    fn compact_admission_status_is_monotonic_and_requires_complete_high_confidence_fields() {
        use super::super::semantic_probe::ProbeResolutionStatus as Raw;
        assert_eq!(
            compact_admitted_status(&complete_compact_output(Raw::Resolved), &[]),
            "resolved"
        );
        assert_eq!(
            compact_admitted_status(&complete_compact_output(Raw::PartlyResolved), &[]),
            "partly_resolved"
        );
        assert_eq!(
            compact_admitted_status(&complete_compact_output(Raw::Unresolved), &[]),
            "unresolved"
        );
        assert_eq!(
            compact_admitted_status(&complete_compact_output(Raw::Refused), &[]),
            "refused"
        );

        let mut missing_action = complete_compact_output(Raw::Resolved);
        missing_action.next_supported_action = None;
        assert_eq!(
            compact_admitted_status(&missing_action, &[]),
            "partly_resolved"
        );

        let mut low_confidence = complete_compact_output(Raw::Resolved);
        low_confidence
            .confidence_by_field
            .insert("unfinished_task".into(), 0.74);
        assert_eq!(
            compact_admitted_status(&low_confidence, &[]),
            "partly_resolved"
        );

        let mut qualified = complete_compact_output(Raw::Resolved);
        qualified.unfinished_task = Some("Likely verify the completed visual cue".into());
        assert_eq!(compact_admitted_status(&qualified, &[]), "partly_resolved");

        assert_eq!(
            compact_admitted_status(
                &complete_compact_output(Raw::Resolved),
                &["p6_compact_task_conflict".into()]
            ),
            "unresolved"
        );
    }

    #[test]
    fn field_admission_reasons_are_typed_without_collapsing_other_fields() {
        use super::super::semantic_probe::{ProbeFieldVerifierResult, ProbeResolutionStatus};
        let mut output = complete_compact_output(ProbeResolutionStatus::Resolved);
        output.resume_point = None;
        output
            .verifier_result_by_field
            .insert("resume_point".into(), ProbeFieldVerifierResult::Rejected);
        let admissions = compact_field_admission(
            &output,
            &[
                "resume_point:stale_slot".into(),
                "visit_role:T1_VISIT:missing".into(),
            ],
        );
        assert_eq!(admissions["resume_point"].verdict, "rejected_stale");
        assert_eq!(admissions["unfinished_task"].verdict, "accepted");
        assert_eq!(admissions["next_supported_action"].verdict, "accepted");
    }
}
