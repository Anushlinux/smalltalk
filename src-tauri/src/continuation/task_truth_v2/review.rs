use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::super::{stable_hash, ContinueDecisionResult};
use super::audit::{performance_review_summary, PerformanceReviewSummaryV1};
use super::production::{TaskTruthAtomicIdentityV1, TaskTruthPublicAnswerV1};

const REVIEW_SCHEMA: &str = "smalltalk.mfti_review.v1";
const MAX_SEMANTIC_CHARS: usize = 320;
const MAX_IDENTIFIER_CHARS: usize = 192;

#[derive(Debug, Clone, Serialize)]
struct ReviewSemanticAnswer {
    resolution_status: String,
    raw_model_status: String,
    admitted_semantic_status: String,
    semantic_source_kind: String,
    target_status: String,
    unresolved_or_failure_reason: Option<String>,
    atomic_answer_identity: String,
    unfinished_task: Option<String>,
    task_state: String,
    resume_point: Option<String>,
    next_supported_action: Option<String>,
    completed_context: Option<String>,
    field_admission: BTreeMap<String, String>,
    claim_confidence: BTreeMap<String, f64>,
    semantic_conflicts: Vec<String>,
    task_summary: Option<String>,
    current_subtask: Option<String>,
    task_object: Option<String>,
    execution_state: String,
    relationship_to_prior: String,
    last_meaningful_progress: Option<String>,
    unfinished_state: Option<String>,
    next_action: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ReviewInference {
    status: String,
    origin: String,
    provider: String,
    model: String,
    request_id: Option<String>,
    provider_request_id: Option<String>,
    response_id: Option<String>,
    provider_attempt_count: usize,
    verification_status: String,
}

#[derive(Debug, Clone, Serialize)]
struct ReviewAtomicIdentity {
    decision_id: String,
    current_frame_id: String,
    packet_policy_version: String,
    response_schema_version: String,
    admission_version: String,
    admitted_result_id: String,
    correction_watermark: String,
    target_identity: Option<String>,
    session_id: Option<String>,
    task_thread_id: Option<String>,
    task_thread_revision: Option<i64>,
    task_snapshot_id: String,
    snapshot_revision: i64,
    selected_hypothesis_id: Option<String>,
    model_request_id: Option<String>,
    model_response_id: Option<String>,
    observation_packet_id: String,
    evidence_watermark: String,
    correction_fingerprint: String,
}

#[derive(Debug, Clone)]
struct PersistedDecisionIdentity {
    observation_packet_id: Option<String>,
    task_snapshot_id: Option<String>,
    snapshot_revision: Option<i64>,
    task_thread_id: Option<String>,
    task_thread_revision: Option<i64>,
    selected_hypothesis_id: Option<String>,
    model_request_id: Option<String>,
    model_response_id: Option<String>,
    provider_attempt_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ReviewSurfaceProjection {
    display_kind: String,
    action_kinds: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ReviewPresentation {
    react: ReviewSurfaceProjection,
    island: ReviewSurfaceProjection,
}

#[derive(Debug, Clone, Serialize)]
struct ReviewPrivacyContract {
    screenshots_included: bool,
    ocr_included: bool,
    accessibility_text_included: bool,
    raw_provider_response_included: bool,
    urls_included: bool,
    file_paths_included: bool,
    credentials_or_keys_included: bool,
    raw_history_included: bool,
}

#[derive(Debug, Clone, Serialize)]
struct MftiReviewArtifact {
    schema: &'static str,
    audit_mode: &'static str,
    generated_at_ms: i64,
    decision_id: String,
    decision_id_hash: String,
    failure_reasons: Vec<String>,
    semantic_answer: ReviewSemanticAnswer,
    inference: ReviewInference,
    atomic_identity: ReviewAtomicIdentity,
    performance: Option<PerformanceReviewSummaryV1>,
    presentation: ReviewPresentation,
    privacy: ReviewPrivacyContract,
}

fn looks_like_disallowed_locator_or_secret(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("://")
        || lower.contains("www.")
        || lower.contains("/users/")
        || lower.contains("/home/")
        || lower.contains("file:")
        || lower.contains("sk-proj-")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("authorization:")
        || lower.contains("bearer ")
}

fn bounded(value: Option<&str>, max_chars: usize) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() || looks_like_disallowed_locator_or_secret(value) {
        return None;
    }
    Some(value.chars().take(max_chars).collect())
}

fn semantic(value: Option<&str>) -> Option<String> {
    bounded(value, MAX_SEMANTIC_CHARS)
}

fn identifier(value: Option<&str>) -> Option<String> {
    bounded(value, MAX_IDENTIFIER_CHARS)
}

fn nonempty_identity(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn canonical_diagnostic_request_id(
    diagnostic: Option<&super::production::TaskTruthInferenceDiagnosticV1>,
) -> Option<&str> {
    diagnostic.and_then(|diagnostic| {
        nonempty_identity(diagnostic.provider_request_id.as_deref())
            .or_else(|| nonempty_identity(diagnostic.request_id.as_deref()))
    })
}

fn inference_identity_matches(
    contract_request_id: Option<&str>,
    answer_request_id: Option<&str>,
    contract_response_id: Option<&str>,
    answer_response_id: Option<&str>,
    diagnostic: Option<&super::production::TaskTruthInferenceDiagnosticV1>,
) -> bool {
    let diagnostic_request_id = canonical_diagnostic_request_id(diagnostic);
    let diagnostic_response_id =
        diagnostic.and_then(|diagnostic| nonempty_identity(diagnostic.response_id.as_deref()));

    contract_request_id == answer_request_id
        && contract_request_id == diagnostic_request_id
        && contract_response_id == answer_response_id
        // Some providers and typed no-response failures do not supply a native
        // response id. The production answer then uses its deterministic
        // envelope identity. The persisted contract and answer must still
        // agree; a native diagnostic id, when present, must also agree.
        && diagnostic_response_id
            .map(|response_id| contract_response_id == Some(response_id))
            .unwrap_or(true)
}

fn safe_label(value: &str, fallback: &str) -> String {
    identifier(Some(value)).unwrap_or_else(|| fallback.to_string())
}

fn semantic_answer(answer: Option<&TaskTruthPublicAnswerV1>) -> ReviewSemanticAnswer {
    let Some(answer) = answer else {
        return ReviewSemanticAnswer {
            resolution_status: "unresolved".to_string(),
            raw_model_status: "unresolved".to_string(),
            admitted_semantic_status: "unresolved".to_string(),
            semantic_source_kind: "unresolved".to_string(),
            target_status: "no_task".to_string(),
            unresolved_or_failure_reason: None,
            atomic_answer_identity: String::new(),
            unfinished_task: None,
            task_state: "unclear".to_string(),
            resume_point: None,
            next_supported_action: None,
            completed_context: None,
            field_admission: BTreeMap::new(),
            claim_confidence: BTreeMap::new(),
            semantic_conflicts: Vec::new(),
            task_summary: None,
            current_subtask: None,
            task_object: None,
            execution_state: "unclear".to_string(),
            relationship_to_prior: "unrelated_or_unknown".to_string(),
            last_meaningful_progress: None,
            unfinished_state: None,
            next_action: None,
        };
    };
    ReviewSemanticAnswer {
        resolution_status: safe_label(&answer.task_resolution_status, "unresolved"),
        raw_model_status: safe_label(&answer.raw_model_status, "unresolved"),
        admitted_semantic_status: safe_label(&answer.admitted_semantic_status, "unresolved"),
        semantic_source_kind: safe_label(&answer.semantic_source_kind, "unresolved"),
        target_status: safe_label(&answer.target_status, "no_task"),
        unresolved_or_failure_reason: identifier(answer.unresolved_or_failure_reason.as_deref()),
        atomic_answer_identity: identifier(Some(&answer.atomic_answer_identity))
            .unwrap_or_default(),
        unfinished_task: semantic(answer.unfinished_task.as_deref()),
        task_state: safe_label(&answer.task_state, "unclear"),
        resume_point: semantic(answer.resume_point.as_deref()),
        next_supported_action: semantic(answer.next_supported_action.as_deref()),
        completed_context: semantic(answer.completed_context.as_deref()),
        field_admission: answer
            .field_admission
            .iter()
            .map(|(field, admission)| {
                (
                    safe_label(field, "unknown_field"),
                    safe_label(&admission.verdict, "rejected_invalid_state"),
                )
            })
            .collect(),
        claim_confidence: answer.claim_confidence.clone(),
        semantic_conflicts: answer
            .semantic_conflicts
            .iter()
            .filter_map(|conflict| identifier(Some(conflict)))
            .collect(),
        task_summary: semantic(answer.task_summary.as_deref()),
        current_subtask: semantic(answer.current_subtask.as_deref()),
        task_object: semantic(answer.task_object.as_deref()),
        execution_state: safe_label(&answer.execution_state, "unclear"),
        relationship_to_prior: safe_label(&answer.relationship_to_prior, "unrelated_or_unknown"),
        last_meaningful_progress: semantic(answer.last_meaningful_progress.as_deref()),
        unfinished_state: semantic(answer.unfinished_state.as_deref()),
        next_action: semantic(answer.next_action.as_deref()),
    }
}

fn atomic_identity(identity: Option<&TaskTruthAtomicIdentityV1>) -> ReviewAtomicIdentity {
    let empty = TaskTruthAtomicIdentityV1::default();
    let identity = identity.unwrap_or(&empty);
    ReviewAtomicIdentity {
        decision_id: identifier(Some(&identity.decision_id)).unwrap_or_default(),
        current_frame_id: identifier(Some(&identity.current_frame_id)).unwrap_or_default(),
        packet_policy_version: identifier(Some(&identity.packet_policy_version))
            .unwrap_or_default(),
        response_schema_version: identifier(Some(&identity.response_schema_version))
            .unwrap_or_default(),
        admission_version: identifier(Some(&identity.admission_version)).unwrap_or_default(),
        admitted_result_id: identifier(Some(&identity.admitted_result_id)).unwrap_or_default(),
        correction_watermark: identifier(Some(&identity.correction_watermark)).unwrap_or_default(),
        target_identity: identifier(identity.target_identity.as_deref()),
        session_id: identifier(identity.session_id.as_deref()),
        task_thread_id: identifier(identity.task_thread_id.as_deref()),
        task_thread_revision: identity.task_thread_revision.filter(|value| *value >= 0),
        task_snapshot_id: identifier(Some(&identity.task_snapshot_id)).unwrap_or_default(),
        snapshot_revision: identity.snapshot_revision.max(0),
        selected_hypothesis_id: identifier(identity.selected_hypothesis_id.as_deref()),
        model_request_id: identifier(identity.model_request_id.as_deref()),
        model_response_id: identifier(identity.model_response_id.as_deref()),
        observation_packet_id: identifier(Some(&identity.observation_packet_id))
            .unwrap_or_default(),
        evidence_watermark: identifier(Some(&identity.evidence_watermark)).unwrap_or_default(),
        correction_fingerprint: identifier(Some(&identity.correction_fingerprint))
            .unwrap_or_default(),
    }
}

fn presentation(answer: Option<&TaskTruthPublicAnswerV1>) -> ReviewPresentation {
    let unresolved = answer.is_none_or(|answer| answer.task_resolution_status == "unresolved");
    let openable = answer
        .and_then(|answer| answer.direct_return_target.as_ref())
        .is_some_and(|target| {
            target.openability == "openable"
                && (target.browser_url.is_some() || target.document_path.is_some())
        });
    if unresolved {
        return ReviewPresentation {
            react: ReviewSurfaceProjection {
                display_kind: "no_clear_continuation".to_string(),
                action_kinds: vec!["inspect_evidence".to_string()],
            },
            island: ReviewSurfaceProjection {
                display_kind: "no_clear_continuation".to_string(),
                action_kinds: vec![
                    "refresh_continue".to_string(),
                    "inspect_evidence".to_string(),
                    "open_smalltalk".to_string(),
                    "capture_evidence_now".to_string(),
                ],
            },
        };
    }
    if openable {
        ReviewPresentation {
            react: ReviewSurfaceProjection {
                display_kind: "openable_return_target".to_string(),
                action_kinds: vec!["continue_here".to_string()],
            },
            island: ReviewSurfaceProjection {
                display_kind: "continue_ready".to_string(),
                action_kinds: vec![
                    "open_continue_target".to_string(),
                    "open_smalltalk".to_string(),
                ],
            },
        }
    } else {
        ReviewPresentation {
            react: ReviewSurfaceProjection {
                display_kind: "thin_current_work".to_string(),
                action_kinds: vec!["inspect_evidence".to_string()],
            },
            island: ReviewSurfaceProjection {
                display_kind: "inspect_only".to_string(),
                action_kinds: vec!["inspect_evidence".to_string(), "open_smalltalk".to_string()],
            },
        }
    }
}

fn review_artifact(
    conn: &Connection,
    decision: &ContinueDecisionResult,
) -> Result<MftiReviewArtifact, String> {
    let answer = decision.task_truth_v2.answer.as_ref();
    let diagnostic = decision.task_truth_v2.inference_diagnostic.as_ref();
    let audit_packet_id = conn
        .query_row(
            "SELECT packet_id FROM task_truth_v2_shadow_audits
             WHERE decision_id=?1 ORDER BY observed_at_ms DESC LIMIT 1",
            params![decision.decision_id.as_str()],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .flatten();
    let contract_identity = conn
        .query_row(
            "SELECT observation_packet_id, snapshot_id, snapshot_revision,
                    task_thread_id, task_thread_revision, selected_hypothesis_id,
                    model_request_id, model_response_id, provider_attempt_count
             FROM task_truth_v2_decision_contracts
             WHERE decision_id=?1",
            params![decision.decision_id.as_str()],
            |row| {
                Ok(PersistedDecisionIdentity {
                    observation_packet_id: row.get(0)?,
                    task_snapshot_id: row.get(1)?,
                    snapshot_revision: row.get(2)?,
                    task_thread_id: row.get(3)?,
                    task_thread_revision: row.get(4)?,
                    selected_hypothesis_id: row.get(5)?,
                    model_request_id: row.get(6)?,
                    model_response_id: row.get(7)?,
                    provider_attempt_count: row.get::<_, i64>(8)?.max(0) as usize,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if audit_packet_id.is_none() {
        return Err("mfti_review_attempt_audit_missing".to_string());
    }
    if contract_identity.is_none() {
        return Err("mfti_review_decision_contract_missing".to_string());
    }
    if answer.is_none() {
        return Err("mfti_review_task_truth_answer_missing".to_string());
    }
    if diagnostic.is_none() {
        return Err("mfti_review_inference_diagnostic_missing".to_string());
    }
    let contract_packet_id = contract_identity
        .as_ref()
        .and_then(|identity| identity.observation_packet_id.as_deref())
        .filter(|value| !value.trim().is_empty());
    let answer_packet_id = answer
        .map(|answer| answer.atomic_identity.observation_packet_id.as_str())
        .filter(|value| !value.trim().is_empty());
    if contract_packet_id != audit_packet_id.as_deref()
        || answer_packet_id != audit_packet_id.as_deref()
    {
        return Err("mfti_review_packet_identity_mismatch".to_string());
    }
    let contract_request_id = contract_identity
        .as_ref()
        .and_then(|identity| nonempty_identity(identity.model_request_id.as_deref()));
    let answer_request_id = answer
        .and_then(|answer| nonempty_identity(answer.atomic_identity.model_request_id.as_deref()));
    let contract_response_id = contract_identity
        .as_ref()
        .and_then(|identity| nonempty_identity(identity.model_response_id.as_deref()));
    let answer_response_id = answer
        .and_then(|answer| nonempty_identity(answer.atomic_identity.model_response_id.as_deref()));
    if !inference_identity_matches(
        contract_request_id,
        answer_request_id,
        contract_response_id,
        answer_response_id,
        diagnostic,
    ) {
        return Err("mfti_review_inference_identity_mismatch".to_string());
    }
    if let (Some(contract), Some(answer)) = (contract_identity.as_ref(), answer) {
        let identity = &answer.atomic_identity;
        let snapshot_revision = nonempty_identity(Some(identity.task_snapshot_id.as_str()))
            .map(|_| identity.snapshot_revision);
        if contract
            .task_snapshot_id
            .as_deref()
            .and_then(|value| nonempty_identity(Some(value)))
            != nonempty_identity(Some(identity.task_snapshot_id.as_str()))
            || contract.snapshot_revision != snapshot_revision
            || contract
                .task_thread_id
                .as_deref()
                .and_then(|value| nonempty_identity(Some(value)))
                != nonempty_identity(identity.task_thread_id.as_deref())
            || contract.task_thread_revision != identity.task_thread_revision
            || contract
                .selected_hypothesis_id
                .as_deref()
                .and_then(|value| nonempty_identity(Some(value)))
                != nonempty_identity(identity.selected_hypothesis_id.as_deref())
        {
            return Err("mfti_review_decision_identity_mismatch".to_string());
        }
    }
    if let (Some(contract), Some(diagnostic)) = (contract_identity.as_ref(), diagnostic) {
        if contract.provider_attempt_count != diagnostic.provider_attempt_count {
            return Err("mfti_review_provider_attempt_count_mismatch".to_string());
        }
    }
    Ok(MftiReviewArtifact {
        schema: REVIEW_SCHEMA,
        audit_mode: "mfti_review",
        generated_at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or_default(),
        decision_id: identifier(Some(&decision.decision_id))
            .unwrap_or_else(|| "redacted_decision_id".to_string()),
        decision_id_hash: stable_hash(decision.decision_id.as_bytes()),
        failure_reasons: decision
            .task_truth_v2
            .reason_codes
            .iter()
            .filter_map(|reason| identifier(Some(reason)))
            .take(12)
            .collect(),
        semantic_answer: semantic_answer(answer),
        inference: ReviewInference {
            status: diagnostic
                .map(|value| safe_label(&value.status, "unavailable"))
                .unwrap_or_else(|| "unavailable".to_string()),
            origin: diagnostic
                .map(|value| safe_label(&value.origin, "none"))
                .unwrap_or_else(|| "none".to_string()),
            provider: diagnostic
                .map(|value| safe_label(&value.provider, "unconfigured"))
                .unwrap_or_else(|| "unconfigured".to_string()),
            model: diagnostic
                .map(|value| safe_label(&value.model, "unconfigured"))
                .unwrap_or_else(|| "unconfigured".to_string()),
            request_id: diagnostic.and_then(|value| identifier(value.request_id.as_deref())),
            provider_request_id: diagnostic
                .and_then(|value| identifier(value.provider_request_id.as_deref())),
            response_id: diagnostic.and_then(|value| identifier(value.response_id.as_deref())),
            provider_attempt_count: diagnostic
                .map(|value| value.provider_attempt_count)
                .unwrap_or_default(),
            verification_status: diagnostic
                .map(|value| safe_label(&value.verification_status, "verification_rejected"))
                .unwrap_or_else(|| "verification_rejected".to_string()),
        },
        atomic_identity: atomic_identity(answer.map(|answer| &answer.atomic_identity)),
        performance: performance_review_summary(conn, &decision.decision_id)?,
        presentation: presentation(answer),
        privacy: ReviewPrivacyContract {
            screenshots_included: false,
            ocr_included: false,
            accessibility_text_included: false,
            raw_provider_response_included: false,
            urls_included: false,
            file_paths_included: false,
            credentials_or_keys_included: false,
            raw_history_included: false,
        },
    })
}

fn default_review_root() -> Result<PathBuf, String> {
    if let Some(root) = std::env::var_os("SMALLTALK_MFTI_REVIEW_ROOT") {
        return Ok(PathBuf::from(root));
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|root| root.join("continue_outputs").join("mfti_review"))
        .ok_or_else(|| "mfti_review_project_root_unavailable".to_string())
}

fn write_artifact_to(
    conn: &Connection,
    decision: &ContinueDecisionResult,
    root: &Path,
) -> Result<PathBuf, String> {
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    let artifact = review_artifact(conn, decision)?;
    let decision_hash = stable_hash(decision.decision_id.as_bytes());
    let final_path = root.join(format!("mfti-review-{decision_hash}.json"));
    let temporary_path = root.join(format!(".mfti-review-{decision_hash}.tmp"));
    let bytes = serde_json::to_vec_pretty(&artifact).map_err(|error| error.to_string())?;
    fs::write(&temporary_path, bytes).map_err(|error| error.to_string())?;
    fs::rename(&temporary_path, &final_path).map_err(|error| error.to_string())?;
    Ok(final_path)
}

pub(crate) fn write_mfti_review_artifact(
    conn: &Connection,
    decision: &ContinueDecisionResult,
) -> Result<(), String> {
    write_artifact_to(conn, decision, &default_review_root()?).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn diagnostic(
        request_id: Option<&str>,
        provider_request_id: Option<&str>,
        response_id: Option<&str>,
    ) -> super::super::production::TaskTruthInferenceDiagnosticV1 {
        super::super::production::TaskTruthInferenceDiagnosticV1 {
            schema: "smalltalk.task_truth_inference_diagnostic.v1".into(),
            status: "success".into(),
            origin: "live_cloud".into(),
            provider: "openai".into(),
            model: "test-model".into(),
            request_id: request_id.map(str::to_string),
            provider_request_id: provider_request_id.map(str::to_string),
            response_id: response_id.map(str::to_string),
            provider_attempt_count: 1,
            latency_ms: 1,
            image_count: 1,
            image_bytes: 1,
            estimated_tokens: 1,
            input_tokens: Some(1),
            output_tokens: Some(1),
            total_tokens: Some(2),
            estimated_cost_usd: Some(0.0),
            verification_status: "verified".into(),
            selected_hypothesis_id: None,
        }
    }

    #[test]
    fn provider_request_identity_is_canonical_for_review_matching() {
        let diagnostic = diagnostic(
            Some("local-request"),
            Some("provider-request"),
            Some("provider-response"),
        );

        assert_eq!(
            canonical_diagnostic_request_id(Some(&diagnostic)),
            Some("provider-request")
        );
        assert!(inference_identity_matches(
            Some("provider-request"),
            Some("provider-request"),
            Some("provider-response"),
            Some("provider-response"),
            Some(&diagnostic),
        ));
        assert!(!inference_identity_matches(
            Some("local-request"),
            Some("local-request"),
            Some("provider-response"),
            Some("provider-response"),
            Some(&diagnostic),
        ));
    }

    #[test]
    fn deterministic_response_envelope_is_valid_when_provider_has_no_native_id() {
        let diagnostic = diagnostic(Some("local-request"), None, None);

        assert!(inference_identity_matches(
            Some("local-request"),
            Some("local-request"),
            Some("provider-response-envelope-hash"),
            Some("provider-response-envelope-hash"),
            Some(&diagnostic),
        ));
        assert!(!inference_identity_matches(
            Some("local-request"),
            Some("local-request"),
            Some("provider-response-envelope-a"),
            Some("provider-response-envelope-b"),
            Some(&diagnostic),
        ));
    }

    #[test]
    fn bounded_text_rejects_paths_urls_and_secrets() {
        for value in [
            "/Users/example/private.txt",
            "https://example.test/private",
            "Authorization: Bearer secret",
            "sk-proj-secret",
            "API_KEY=secret",
        ] {
            assert_eq!(semantic(Some(value)), None);
        }
        assert_eq!(
            semantic(Some("Review the bounded task")),
            Some("Review the bounded task".into())
        );
        assert_eq!(
            semantic(Some(&"x".repeat(400))).unwrap().chars().count(),
            320
        );
    }

    #[test]
    fn serialized_schema_has_no_raw_or_locator_fields() {
        let artifact = MftiReviewArtifact {
            schema: REVIEW_SCHEMA,
            audit_mode: "mfti_review",
            generated_at_ms: 1,
            decision_id: "decision-1".into(),
            decision_id_hash: "hash".into(),
            failure_reasons: vec!["provider_failure".into()],
            semantic_answer: semantic_answer(None),
            inference: ReviewInference {
                status: "provider_error".into(),
                origin: "live_cloud".into(),
                provider: "openai".into(),
                model: "model".into(),
                request_id: Some("request-1".into()),
                provider_request_id: Some("provider-request-1".into()),
                response_id: None,
                provider_attempt_count: 1,
                verification_status: "verification_rejected".into(),
            },
            atomic_identity: atomic_identity(None),
            performance: None,
            presentation: presentation(None),
            privacy: ReviewPrivacyContract {
                screenshots_included: false,
                ocr_included: false,
                accessibility_text_included: false,
                raw_provider_response_included: false,
                urls_included: false,
                file_paths_included: false,
                credentials_or_keys_included: false,
                raw_history_included: false,
            },
        };
        let value = serde_json::to_value(artifact).unwrap();
        let object = value.as_object().unwrap();
        assert_eq!(
            object.get("schema").and_then(Value::as_str),
            Some(REVIEW_SCHEMA)
        );
        let encoded = serde_json::to_string(object).unwrap().to_ascii_lowercase();
        for forbidden in [
            "screenshot_path",
            "ocr_text",
            "accessibility_text\"",
            "raw_response",
            "browser_url",
            "document_path",
            "api_key",
            "raw_history\"",
        ] {
            assert!(!encoded.contains(forbidden), "forbidden field: {forbidden}");
        }
        assert!(encoded.len() < 16_384);
    }
}
