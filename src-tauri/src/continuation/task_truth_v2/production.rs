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

pub(crate) const TASK_TRUTH_PUBLIC_ANSWER_SCHEMA_V1: &str = "smalltalk.task_truth_public_answer.v2";
pub(crate) const TASK_TRUTH_AUTHORITY_POLICY_V1: &str =
    "smalltalk.model_first_task_truth_authority_policy.v1";

fn default_public_task_basis() -> String {
    "unresolved".into()
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskTruthPublicAnswerV1 {
    pub schema: String,
    #[serde(default = "default_public_task_basis")]
    pub task_basis: String,
    pub task_resolution_status: String,
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
    answer.field_support.clear();
    answer.task_understanding_source = "unresolved".into();
    answer.semantic_source = "unresolved".into();
    answer.selected_hypothesis_id = None;
    answer.inference_status = reason.into();
    answer.atomic_identity.selected_hypothesis_id = None;
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
    let model_identity_present = answer.semantic_source != "cloud_multimodal_model"
        || (answer
            .response_id
            .as_deref()
            .is_some_and(|id| !id.trim().is_empty())
            && answer
                .atomic_identity
                .model_response_id
                .as_deref()
                .is_some_and(|id| !id.trim().is_empty()));
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

fn typed_unresolved_answer(
    session_id: Option<&str>,
    diagnostic: Option<&TaskTruthInferenceDiagnosticV1>,
) -> TaskTruthPublicAnswerV1 {
    let inference_status = diagnostic
        .map(|diagnostic| diagnostic.status.clone())
        .unwrap_or_else(|| "no_verified_snapshot".into());
    TaskTruthPublicAnswerV1 {
        inference_status,
        provider_name: diagnostic.map(|diagnostic| diagnostic.provider.clone()),
        provider_model: diagnostic.map(|diagnostic| diagnostic.model.clone()),
        request_id: diagnostic.and_then(|diagnostic| diagnostic.request_id.clone()),
        response_id: diagnostic.and_then(|diagnostic| diagnostic.response_id.clone()),
        atomic_identity: TaskTruthAtomicIdentityV1 {
            session_id: session_id.map(str::to_string),
            ..Default::default()
        },
        ..Default::default()
    }
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
    TaskTruthPublicAnswerV1 {
        schema: TASK_TRUTH_PUBLIC_ANSWER_SCHEMA_V1.into(),
        task_basis: snapshot.task_basis.clone(),
        task_resolution_status: task_resolution_status.into(),
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
        },
    }
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
        answer.atomic_identity.correction_fingerprint = correction_fingerprint;
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
                    answer.selected_hypothesis_id = Some(hypothesis_id.clone());
                    answer.atomic_identity.selected_hypothesis_id = Some(hypothesis_id.clone());
                    answer.field_support.insert(
                        "task_summary".into(),
                        TaskTruthFieldSupportV1 {
                            confidence: Some(1.0),
                            support_status: "human_corrected".into(),
                            evidence_refs: selected.evidence_refs,
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
                answer.task_understanding_source = "human_correction".into();
                answer.semantic_source = "human_correction".into();
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
            });
        match field.as_str() {
            "task_summary" => {
                answer.task_resolution_status = "unresolved".into();
                answer.task_summary = None;
                answer.task_object = None;
                answer.last_meaningful_progress = None;
                answer.unfinished_state = None;
                answer.next_action = None;
                answer.direct_return_target = None;
                answer.task_understanding_source = "unresolved".into();
            }
            "task_object" => answer.task_object = None,
            "state" => {
                answer.last_meaningful_progress = None;
                answer.unfinished_state = None;
            }
            "next_action" => answer.next_action = None,
            "where" => {
                answer.where_summary = None;
                answer.direct_return_target = None;
            }
            _ => {}
        }
    }
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
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
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
    let mut answer = answer_snapshot.map(public_answer);
    if let Some(answer) = answer.as_mut() {
        apply_scoped_feedback(conn, answer)?;
        if let Some(reason) = enforce_model_first_semantic_authority(answer) {
            reason_codes.push(reason);
        }
    }
    let diagnostic_packet_id = attempt_packet_id.or_else(|| {
        answer_snapshot
            .or(newest)
            .or(selected)
            .map(|snapshot| snapshot.packet_id.as_str())
    });
    let inference_diagnostic = inference_diagnostic(conn, diagnostic_packet_id, decision_id)?;
    if answer.is_none() && model_first_answer_required {
        reason_codes.push("typed_unresolved_model_first_answer".into());
        answer = Some(typed_unresolved_answer(
            session_id,
            inference_diagnostic.as_ref(),
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
        if identity_matches && direct_target_allowed && feedback_allows_target {
            answer.direct_return_target = target;
        } else {
            answer.direct_return_target = None;
            if !identity_matches {
                decision
                    .reason_codes
                    .push("target_task_identity_mismatch".into());
            } else if !feedback_allows_target {
                decision
                    .reason_codes
                    .push("target_blocked_by_scoped_task_feedback".into());
            }
        }
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
    conn.execute(
        "INSERT OR REPLACE INTO task_truth_v2_decision_contracts (
           decision_id, effective_state, release_gate_passed, snapshot_id,
           snapshot_revision, task_thread_id, task_thread_revision,
           selected_hypothesis_id, model_request_id, model_response_id,
           provider_attempt_count, observation_packet_id, evidence_watermark,
           correction_fingerprint, return_target_artifact_id, created_at_ms
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,
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
            nonempty_identity(answer.and_then(|answer| answer.atomic_identity.task_thread_id.as_deref())),
            answer.and_then(|answer| answer.atomic_identity.task_thread_revision),
            nonempty_identity(answer.and_then(|answer| answer.atomic_identity.selected_hypothesis_id.as_deref())),
            nonempty_identity(answer.and_then(|answer| answer.atomic_identity.model_request_id.as_deref())),
            nonempty_identity(answer.and_then(|answer| answer.atomic_identity.model_response_id.as_deref())),
            decision
                .inference_diagnostic
                .as_ref()
                .map(|diagnostic| diagnostic.provider_attempt_count as i64)
                .unwrap_or_default(),
            nonempty_identity(answer.map(|answer| answer.atomic_identity.observation_packet_id.as_str())),
            nonempty_identity(answer.map(|answer| answer.atomic_identity.evidence_watermark.as_str())),
            nonempty_identity(answer.map(|answer| answer.atomic_identity.correction_fingerprint.as_str())),
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
    fn manual_production_decision_uses_the_exact_attempt_packet() {
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
        let mut same_packet_newer = unresolved_snapshot(&packet, Some(&snapshot), "same_packet_newer");
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
        let answer = decision.answer.unwrap();
        assert_eq!(
            answer.atomic_identity.observation_packet_id,
            "packet-feedback"
        );
        assert_eq!(answer.snapshot_id, "snapshot-attempt");
        assert_eq!(answer.response_id.as_deref(), Some("response-attempt"));
        let diagnostic = decision.inference_diagnostic.unwrap();
        assert_eq!(diagnostic.provider, "openai");
        assert_eq!(diagnostic.response_id.as_deref(), Some("response-attempt"));
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
}
