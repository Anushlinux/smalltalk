use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use super::observation_packet::{
    AuthorshipStatusV2, EvidenceHandleV2, EvidencePartitionV2, ObservationPacketV2, RegionRoleV2,
};
use super::task_snapshot::TaskSnapshotV2;

pub(crate) const TASK_TRUTH_MODEL_OUTPUT_SCHEMA_V1: &str = "smalltalk.task_truth_model_output.v2";
pub(crate) const TASK_TRUTH_RESOLVER_VERSION: &str = "task_truth_v2.multimodal_resolver.v2";

const MAX_IMAGES: usize = 4;
const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
const MAX_TOTAL_IMAGE_BYTES: usize = 12 * 1024 * 1024;
pub(crate) const MODEL_SEMANTIC_FIELDS: [&str; 16] = [
    "observed_surface",
    "immediate_user_operation",
    "semantic_effect_of_operation",
    "current_subtask",
    "likely_primary_task",
    "task_object",
    "app_identity",
    "surface_identity_hash",
    "document_or_thread_identity_hash",
    "execution_state",
    "current_actor",
    "waiting_on",
    "last_meaningful_progress",
    "unfinished_state",
    "possible_next_action",
    "relationship_to_prior",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ResolutionStatusV1 {
    Resolved,
    Ambiguous,
    InsufficientEvidence,
    PrivacyBlocked,
    ModelUnavailable,
    ProviderFailure,
    InvalidResponse,
    VerificationRejected,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderDiagnosticStatusV1 {
    Disabled,
    CredentialsMissing,
    ModelUnavailable,
    PrivacyBlocked,
    RequestInvalid,
    RequestRejected,
    Timeout,
    ProviderError,
    InvalidResponse,
    VerificationRejected,
    Success,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InferenceOriginV1 {
    LiveCloud,
    Cache,
    Fixture,
    None,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskRelationshipV1 {
    Continuation,
    SupportingResearch,
    Verification,
    TemporaryDetour,
    Interruption,
    NewTask,
    ReturnToPriorTask,
    UnrelatedOrUnknown,
}

impl TaskRelationshipV1 {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Continuation => "continuation",
            Self::SupportingResearch => "supporting_research",
            Self::Verification => "verification",
            Self::TemporaryDetour => "temporary_detour",
            Self::Interruption => "interruption",
            Self::NewTask => "new_task",
            Self::ReturnToPriorTask => "return_to_prior_task",
            Self::UnrelatedOrUnknown => "unrelated_or_unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ModelClaimEvidenceV1 {
    pub(crate) claim: String,
    pub(crate) evidence_refs: Vec<EvidenceHandleV2>,
    pub(crate) confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct FieldContradictionV1 {
    pub(crate) field: String,
    pub(crate) reason: String,
    pub(crate) evidence_refs: Vec<EvidenceHandleV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ModelTaskHypothesisV1 {
    pub(crate) hypothesis_id: String,
    pub(crate) observed_surface: Option<String>,
    pub(crate) immediate_user_operation: Option<String>,
    pub(crate) semantic_effect_of_operation: Option<String>,
    pub(crate) current_subtask: Option<String>,
    pub(crate) likely_primary_task: Option<String>,
    pub(crate) task_object: Option<String>,
    pub(crate) app_identity: Option<String>,
    pub(crate) surface_identity_hash: Option<String>,
    pub(crate) document_or_thread_identity_hash: Option<String>,
    pub(crate) execution_state: Option<String>,
    pub(crate) current_actor: Option<String>,
    pub(crate) waiting_on: Option<String>,
    pub(crate) last_meaningful_progress: Option<String>,
    pub(crate) unfinished_state: Option<String>,
    pub(crate) possible_next_action: Option<String>,
    pub(crate) relationship_to_prior: TaskRelationshipV1,
    pub(crate) continuity_thread_id: Option<String>,
    pub(crate) continuity_thread_revision: Option<i64>,
    pub(crate) continuity_identity_token: Option<String>,
    pub(crate) supersedes_thread_id: Option<String>,
    pub(crate) return_anchor_record_id: Option<String>,
    pub(crate) claim_evidence: std::collections::BTreeMap<String, Option<ModelClaimEvidenceV1>>,
    pub(crate) contradictions: Vec<FieldContradictionV1>,
    pub(crate) confidence_by_field: std::collections::BTreeMap<String, f64>,
    pub(crate) confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct PriorTaskThreadContextV1 {
    pub(crate) task_thread_id: String,
    pub(crate) identity_token: String,
    pub(crate) revision: i64,
    pub(crate) status: String,
    pub(crate) current_session_id: String,
    pub(crate) session_lineage: Vec<String>,
    pub(crate) head_snapshot_id: String,
    pub(crate) task_summary: Option<String>,
    pub(crate) task_object: Option<String>,
    pub(crate) execution_state: String,
    pub(crate) last_meaningful_progress: Option<String>,
    pub(crate) unfinished_state: Option<String>,
    pub(crate) last_supported_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct TaskTruthModelOutputV1 {
    pub(crate) schema: String,
    pub(crate) resolution_status: ResolutionStatusV1,
    pub(crate) hypotheses: Vec<ModelTaskHypothesisV1>,
    pub(crate) missing_evidence: Vec<String>,
    pub(crate) policy_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ModelFailureKindV1 {
    Unavailable,
    RequestInvalid,
    RequestRejected,
    Timeout,
    InvalidJson,
    PolicyRefusal,
    ProviderError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ModelFailureV1 {
    pub(crate) kind: ModelFailureKindV1,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ProviderUsageV1 {
    pub(crate) input_tokens: Option<i64>,
    pub(crate) output_tokens: Option<i64>,
    pub(crate) total_tokens: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ProviderAttemptMetadataV1 {
    pub(crate) response_id: Option<String>,
    pub(crate) request_id: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) usage: ProviderUsageV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct TaskTruthModelResponseV1 {
    pub(crate) output: TaskTruthModelOutputV1,
    pub(crate) provider_response_id: Option<String>,
    pub(crate) provider_request_id: Option<String>,
    pub(crate) provider_model: Option<String>,
    pub(crate) usage: ProviderUsageV1,
    pub(crate) provider_attempts: Vec<ProviderAttemptMetadataV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct MultimodalRequestAuditV1 {
    pub(crate) request_schema: String,
    pub(crate) provider: String,
    pub(crate) model: String,
    pub(crate) request_id: String,
    pub(crate) image_count: usize,
    pub(crate) image_bytes: usize,
    pub(crate) image_handle_hashes: Vec<String>,
    pub(crate) skipped_images: Vec<String>,
    pub(crate) structured_bytes: usize,
    pub(crate) estimated_tokens: usize,
    pub(crate) max_images: usize,
    pub(crate) max_image_bytes: usize,
    pub(crate) privacy_exclusions: Vec<String>,
    pub(crate) current_frame_readable_visual: bool,
    pub(crate) current_frame_visual_reason: Option<String>,
    pub(crate) supplied_images: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct TaskTruthModelRequestV1 {
    pub(crate) body: Value,
    pub(crate) audit: MultimodalRequestAuditV1,
    pub(crate) evidence_catalog: BTreeMap<String, EvidenceHandleV2>,
    pub(crate) evidence_policy: EvidenceKeyPolicyV1,
    pub(crate) prior_threads: Vec<PriorTaskThreadContextV1>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct EvidenceKeyPolicyV1 {
    causal: BTreeSet<String>,
    delta: BTreeSet<String>,
    prior: BTreeSet<String>,
    task_identity: BTreeSet<String>,
    task_object: BTreeSet<String>,
    user_plan: BTreeSet<String>,
}

pub(crate) trait TaskTruthModelClient {
    fn provider_name(&self) -> &str;
    fn model_name(&self) -> &str;
    fn inference_origin(&self) -> InferenceOriginV1;
    fn provider_attempts(&self) -> Vec<ProviderAttemptMetadataV1> {
        Vec::new()
    }
    fn infer(
        &self,
        request: &TaskTruthModelRequestV1,
    ) -> Result<TaskTruthModelResponseV1, ModelFailureV1>;
}

pub(crate) trait TaskTruthResolver {
    fn resolve(
        &self,
        packet: &ObservationPacketV2,
        prior: Option<&TaskSnapshotV2>,
        prior_threads: &[PriorTaskThreadContextV1],
        client: &dyn TaskTruthModelClient,
    ) -> ResolverAttemptV1;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ResolverAttemptV1 {
    pub(crate) status: ResolutionStatusV1,
    pub(crate) diagnostic_status: ProviderDiagnosticStatusV1,
    pub(crate) origin: InferenceOriginV1,
    pub(crate) provider: String,
    pub(crate) model: String,
    pub(crate) request_id: Option<String>,
    pub(crate) provider_request_id: Option<String>,
    pub(crate) response_id: Option<String>,
    pub(crate) usage: ProviderUsageV1,
    #[serde(default)]
    pub(crate) provider_attempts: Vec<ProviderAttemptMetadataV1>,
    pub(crate) output: Option<TaskTruthModelOutputV1>,
    pub(crate) failure: Option<ModelFailureV1>,
    pub(crate) request_audit: Option<MultimodalRequestAuditV1>,
    #[serde(default)]
    pub(crate) request_build_latency_ms: i64,
    #[serde(default)]
    pub(crate) provider_latency_ms: i64,
    pub(crate) latency_ms: i64,
}

pub(crate) struct MultimodalTaskTruthResolver;

pub(crate) fn diagnostic_for_failure(failure: &ModelFailureV1) -> ProviderDiagnosticStatusV1 {
    match failure.kind {
        ModelFailureKindV1::Unavailable if failure.reason.trim() == "multimodal_disabled" => {
            ProviderDiagnosticStatusV1::Disabled
        }
        ModelFailureKindV1::Unavailable if failure.reason.trim() == "credentials_missing" => {
            ProviderDiagnosticStatusV1::CredentialsMissing
        }
        ModelFailureKindV1::Unavailable => ProviderDiagnosticStatusV1::ModelUnavailable,
        ModelFailureKindV1::RequestInvalid => ProviderDiagnosticStatusV1::RequestInvalid,
        ModelFailureKindV1::RequestRejected | ModelFailureKindV1::PolicyRefusal => {
            ProviderDiagnosticStatusV1::RequestRejected
        }
        ModelFailureKindV1::Timeout => ProviderDiagnosticStatusV1::Timeout,
        ModelFailureKindV1::InvalidJson => ProviderDiagnosticStatusV1::InvalidResponse,
        ModelFailureKindV1::ProviderError => ProviderDiagnosticStatusV1::ProviderError,
    }
}

pub(crate) fn resolution_for_failure(failure: &ModelFailureV1) -> ResolutionStatusV1 {
    match failure.kind {
        ModelFailureKindV1::Unavailable => ResolutionStatusV1::ModelUnavailable,
        ModelFailureKindV1::InvalidJson => ResolutionStatusV1::InvalidResponse,
        ModelFailureKindV1::RequestInvalid => ResolutionStatusV1::InvalidResponse,
        ModelFailureKindV1::Timeout
        | ModelFailureKindV1::RequestRejected
        | ModelFailureKindV1::PolicyRefusal
        | ModelFailureKindV1::ProviderError => ResolutionStatusV1::ProviderFailure,
    }
}

impl TaskTruthResolver for MultimodalTaskTruthResolver {
    fn resolve(
        &self,
        packet: &ObservationPacketV2,
        prior: Option<&TaskSnapshotV2>,
        prior_threads: &[PriorTaskThreadContextV1],
        client: &dyn TaskTruthModelClient,
    ) -> ResolverAttemptV1 {
        let started = Instant::now();
        let provider = client.provider_name().to_string();
        let model = client.model_name().to_string();
        let origin = client.inference_origin();
        if !packet.current_frame.model_eligible {
            let privacy_blocked = super::observation_packet::is_private_status(Some(
                packet.current_frame.privacy_status.as_str(),
            ));
            return ResolverAttemptV1 {
                status: if privacy_blocked {
                    ResolutionStatusV1::PrivacyBlocked
                } else {
                    ResolutionStatusV1::InsufficientEvidence
                },
                diagnostic_status: if privacy_blocked {
                    ProviderDiagnosticStatusV1::PrivacyBlocked
                } else {
                    ProviderDiagnosticStatusV1::RequestInvalid
                },
                origin,
                provider,
                model,
                request_id: None,
                provider_request_id: None,
                response_id: None,
                usage: ProviderUsageV1::default(),
                provider_attempts: Vec::new(),
                output: None,
                failure: (!privacy_blocked).then(|| ModelFailureV1 {
                    kind: ModelFailureKindV1::Unavailable,
                    reason: packet
                        .current_frame
                        .image_rejection_reason
                        .clone()
                        .unwrap_or_else(|| "current_frame_missing_readable_visual".into()),
                }),
                request_audit: None,
                request_build_latency_ms: 0,
                provider_latency_ms: 0,
                latency_ms: started.elapsed().as_millis() as i64,
            };
        }
        let request_build_started = Instant::now();
        let request =
            match build_multimodal_request(packet, prior, prior_threads, client.model_name(), None)
            {
                Ok(request) => request,
                Err(failure) => {
                    return ResolverAttemptV1 {
                        status: resolution_for_failure(&failure),
                        diagnostic_status: diagnostic_for_failure(&failure),
                        origin,
                        provider,
                        model,
                        request_id: None,
                        provider_request_id: None,
                        response_id: None,
                        usage: ProviderUsageV1::default(),
                        provider_attempts: Vec::new(),
                        output: None,
                        failure: Some(failure),
                        request_audit: None,
                        request_build_latency_ms: request_build_started.elapsed().as_millis()
                            as i64,
                        provider_latency_ms: 0,
                        latency_ms: started.elapsed().as_millis() as i64,
                    }
                }
            };
        let request_build_latency_ms = request_build_started.elapsed().as_millis() as i64;
        let audit = request.audit.clone();
        let provider_started = Instant::now();
        match client.infer(&request) {
            Ok(response) => ResolverAttemptV1 {
                status: response.output.resolution_status,
                diagnostic_status: ProviderDiagnosticStatusV1::Success,
                origin,
                provider,
                model,
                request_id: Some(audit.request_id.clone()),
                provider_request_id: response.provider_request_id,
                response_id: response.provider_response_id,
                usage: response.usage,
                provider_attempts: response.provider_attempts,
                output: Some(response.output),
                failure: None,
                request_audit: Some(audit),
                request_build_latency_ms,
                provider_latency_ms: provider_started.elapsed().as_millis() as i64,
                latency_ms: started.elapsed().as_millis() as i64,
            },
            Err(failure) => {
                let provider_attempts = client.provider_attempts();
                let provider_attempt = provider_attempts.last().cloned().unwrap_or_default();
                let usage = aggregate_provider_usage(&provider_attempts);
                ResolverAttemptV1 {
                    status: resolution_for_failure(&failure),
                    diagnostic_status: diagnostic_for_failure(&failure),
                    origin,
                    provider,
                    model,
                    request_id: Some(audit.request_id.clone()),
                    provider_request_id: provider_attempt.request_id,
                    response_id: provider_attempt.response_id,
                    usage,
                    provider_attempts,
                    output: None,
                    failure: Some(failure),
                    request_audit: Some(audit),
                    request_build_latency_ms,
                    provider_latency_ms: provider_started.elapsed().as_millis() as i64,
                    latency_ms: started.elapsed().as_millis() as i64,
                }
            }
        }
    }
}

fn mime_type(path: &Path) -> Option<&'static str> {
    match path
        .extension()?
        .to_string_lossy()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

pub(super) fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0] as u32;
        let b = chunk.get(1).copied().unwrap_or(0) as u32;
        let c = chunk.get(2).copied().unwrap_or(0) as u32;
        let value = (a << 16) | (b << 8) | c;
        output.push(TABLE[((value >> 18) & 63) as usize] as char);
        output.push(TABLE[((value >> 12) & 63) as usize] as char);
        output.push(if chunk.len() > 1 {
            TABLE[((value >> 6) & 63) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[(value & 63) as usize] as char
        } else {
            '='
        });
    }
    output
}

pub(super) fn read_model_image(
    frame: &super::observation_packet::KeyframeReferenceV2,
) -> Result<(Vec<u8>, String), String> {
    let raw_path = frame
        .ephemeral_local_image_path
        .as_deref()
        .ok_or_else(|| "missing_ephemeral_image".to_string())?;
    if let Some(crop) = frame.crop_pixels.as_ref() {
        let output = std::env::temp_dir().join(format!(
            "smalltalk-mfti-crop-{}-{}.jpg",
            std::process::id(),
            frame.frame_id
        ));
        let result = Command::new("/usr/bin/sips")
            .args([
                "-c",
                &format!("{}", crop.height.round() as i64),
                &format!("{}", crop.width.round() as i64),
                "--cropOffset",
                &format!("{}", crop.y.round() as i64),
                &format!("{}", crop.x.round() as i64),
            ])
            .arg(raw_path)
            .arg("--out")
            .arg(&output)
            .output()
            .map_err(|_| "derived_crop_tool_unavailable".to_string())?;
        if !result.status.success() || !output.is_file() {
            let _ = fs::remove_file(&output);
            return Err("derived_crop_failed".into());
        }
        let bytes = fs::read(&output).map_err(|_| "derived_crop_unreadable".to_string())?;
        let _ = fs::remove_file(&output);
        return Ok((bytes, "image/jpeg".into()));
    }
    let path = Path::new(raw_path);
    let mime = mime_type(path).ok_or_else(|| "unsupported_image_type".to_string())?;
    fs::read(path)
        .map(|bytes| (bytes, mime.into()))
        .map_err(|_| "unreadable_image".into())
}

fn build_evidence_reference_catalog(
    packet: &ObservationPacketV2,
    prior: Option<&TaskSnapshotV2>,
    prior_threads: &[PriorTaskThreadContextV1],
) -> BTreeMap<String, EvidenceHandleV2> {
    let mut references = Vec::new();
    references.extend(
        packet
            .canonical_elements
            .iter()
            .map(|element| EvidenceHandleV2 {
                source_kind: "canonical_element".into(),
                record_id: element.element_id.clone(),
                frame_id: Some(element.frame_id.clone()),
                content_hash: element.text_reference.clone(),
            }),
    );
    references.extend(packet.causal_events.iter().map(|event| EvidenceHandleV2 {
        source_kind: "causal_event".into(),
        record_id: event.event_id.clone(),
        frame_id: Some(event.frame_id.clone()),
        content_hash: None,
    }));
    references.extend(
        packet
            .semantic_keyframes
            .iter()
            .map(|frame| EvidenceHandleV2 {
                source_kind: "keyframe".into(),
                record_id: frame.frame_id.clone(),
                frame_id: Some(frame.frame_id.clone()),
                content_hash: frame.local_image_handle_hash.clone(),
            }),
    );
    references.extend(packet.frame_changes.iter().map(|delta| EvidenceHandleV2 {
        source_kind: "semantic_delta".into(),
        record_id: delta.delta_id.clone(),
        frame_id: Some(delta.frame_id.clone()),
        content_hash: None,
    }));
    references.extend(packet.transition_ids.iter().map(|id| EvidenceHandleV2 {
        source_kind: "transition".into(),
        record_id: id.clone(),
        frame_id: None,
        content_hash: None,
    }));
    references.extend(
        packet
            .capture_trigger_ids
            .iter()
            .map(|id| EvidenceHandleV2 {
                source_kind: "capture_trigger".into(),
                record_id: id.clone(),
                frame_id: None,
                content_hash: None,
            }),
    );
    references.extend(
        packet
            .return_anchor_facts
            .iter()
            .map(|anchor| EvidenceHandleV2 {
                source_kind: "return_anchor_fact".into(),
                record_id: anchor.record_id.clone(),
                frame_id: anchor.frame_id.clone(),
                content_hash: anchor.content_hash.clone(),
            }),
    );
    if let Some(prior) = prior.filter(|prior| {
        packet.previous_valid_snapshot_id.as_deref() == Some(prior.snapshot_id.as_str())
    }) {
        references.push(EvidenceHandleV2 {
            source_kind: "prior_snapshot".into(),
            record_id: prior.snapshot_id.clone(),
            frame_id: None,
            content_hash: None,
        });
    }
    references.extend(prior_threads.iter().map(|thread| EvidenceHandleV2 {
        source_kind: "prior_thread_revision".into(),
        record_id: format!("{}:{}", thread.task_thread_id, thread.revision),
        frame_id: None,
        content_hash: Some(thread.identity_token.clone()),
    }));
    references.sort_by(|left, right| {
        left.source_kind
            .cmp(&right.source_kind)
            .then_with(|| left.record_id.cmp(&right.record_id))
            .then_with(|| left.frame_id.cmp(&right.frame_id))
            .then_with(|| left.content_hash.cmp(&right.content_hash))
    });
    references.dedup();
    references
        .into_iter()
        .enumerate()
        .map(|(index, reference)| (format!("evidence_{:04}", index + 1), reference))
        .collect()
}

fn evidence_keys_for_source(
    catalog: &BTreeMap<String, EvidenceHandleV2>,
    source_kind: &str,
) -> Vec<String> {
    catalog
        .iter()
        .filter(|(_, reference)| reference.source_kind == source_kind)
        .map(|(key, _)| key.clone())
        .collect()
}

pub(crate) fn build_multimodal_request(
    packet: &ObservationPacketV2,
    prior: Option<&TaskSnapshotV2>,
    prior_threads: &[PriorTaskThreadContextV1],
    model: &str,
    reconciliation: Option<&Value>,
) -> Result<TaskTruthModelRequestV1, ModelFailureV1> {
    let mut evidence_catalog = build_evidence_reference_catalog(packet, prior, prior_threads);
    let causal_keys = evidence_keys_for_source(&evidence_catalog, "causal_event");
    let delta_keys = evidence_keys_for_source(&evidence_catalog, "semantic_delta");
    let mut prior_keys = evidence_keys_for_source(&evidence_catalog, "prior_snapshot");
    prior_keys.extend(evidence_keys_for_source(
        &evidence_catalog,
        "prior_thread_revision",
    ));
    let user_authored_or_causal_keys = evidence_catalog
        .iter()
        .filter(|(_, reference)| {
            reference.source_kind == "causal_event"
                || packet
                    .canonical_elements
                    .iter()
                    .find(|element| element.element_id == reference.record_id)
                    .is_some_and(|element| {
                        element.authorship_status == AuthorshipStatusV2::User
                            || !element.causal_evidence_refs.is_empty()
                    })
        })
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    let hashed_object_keys = evidence_catalog
        .iter()
        .filter(|(_, reference)| {
            reference.source_kind == "canonical_element"
                && reference.content_hash.is_some()
                && packet
                    .canonical_elements
                    .iter()
                    .find(|element| element.element_id == reference.record_id)
                    .is_some_and(|element| element.task_eligible)
        })
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    let user_plan_keys = evidence_catalog
        .iter()
        .filter(|(_, reference)| {
            packet
                .canonical_elements
                .iter()
                .find(|element| element.element_id == reference.record_id)
                .is_some_and(|element| {
                    element.task_eligible && element.authorship_status == AuthorshipStatusV2::User
                })
        })
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    let request_id = format!(
        "task-truth-request-{}",
        super::super::stable_hash(
            format!(
                "{}:{}:{model}:{}",
                packet.packet_id,
                packet.evidence_watermark,
                serde_json::to_string(prior_threads).unwrap_or_default()
            )
            .as_bytes()
        )
    );
    let prior_task_hypothesis = prior.and_then(|prior| {
        (packet.previous_valid_snapshot_id.as_deref() == Some(prior.snapshot_id.as_str())).then(
            || {
                json!({
                    "status": "same_scoped_history_hypothesis_not_truth",
                    "snapshot": prior,
                    "may_not_override_newer_contradictory_evidence": true
                })
            },
        )
    });
    let mut structured = json!({
        "packet": packet,
        "prior_task_hypothesis": prior_task_hypothesis,
        "prior_task_threads": prior_threads,
        "reconciliation": reconciliation,
        "evidence_reference_catalog": evidence_catalog,
        "request_policy": {
            "explicit_manual_continue": true,
            "background_upload_allowed": false,
            "target_selection_authority": false,
            "image_scope_is_explicit_per_frame": true,
            "evidence_reference_rules": {
                "response_shape": "return only opaque keys from evidence_reference_catalog; never return record ids, frame ids, or hashes directly",
                "immediate_user_operation_requires_one_of": causal_keys,
                "semantic_effect_requires_one_of": delta_keys,
                "task_identity_requires_one_of": user_authored_or_causal_keys,
                "task_object_also_requires_one_of": hashed_object_keys,
                "continuity_or_return_requires_prior_plus_current": prior_keys,
                "next_action_requires_user_authored_plan_key": user_plan_keys,
                "unsupported_field_policy": "return the semantic field and its claim_evidence entry as null when its required key set is empty"
            }
        }
    });
    let mut content = Vec::new();
    let mut image_bytes = 0usize;
    let mut image_count = 0usize;
    let mut image_hashes = Vec::new();
    let mut skipped = Vec::new();
    let mut privacy_exclusions = Vec::new();
    let mut supplied_images = Vec::new();
    let mut supplied_frame_ids = BTreeSet::new();
    for frame in &packet.semantic_keyframes {
        if image_count >= MAX_IMAGES {
            skipped.push(format!("{}:frame_cap", frame.frame_id));
            continue;
        }
        if frame.partition == EvidencePartitionV2::Background {
            privacy_exclusions.push(format!("{}:background_display", frame.frame_id));
            continue;
        }
        if !frame.model_eligible {
            let reason = frame
                .image_rejection_reason
                .as_deref()
                .unwrap_or("not_model_eligible");
            skipped.push(format!("{}:{}", frame.frame_id, reason));
            if super::observation_packet::is_private_status(Some(&frame.privacy_status)) {
                privacy_exclusions.push(format!("{}:{reason}", frame.frame_id));
            }
            continue;
        }
        let (bytes, mime) = match read_model_image(frame) {
            Ok(image) => image,
            Err(reason) => {
                skipped.push(format!("{}:{reason}", frame.frame_id));
                continue;
            }
        };
        if bytes.len() > MAX_IMAGE_BYTES || image_bytes + bytes.len() > MAX_TOTAL_IMAGE_BYTES {
            skipped.push(format!("{}:image_byte_cap", frame.frame_id));
            continue;
        }
        content.push(json!({
            "type": "input_text",
            "text": format!(
                "keyframe_id={} observed_at_ms={} partition={:?} surface_identity={} ownership_confidence={:.2}",
                frame.frame_id,
                frame.observed_at_ms,
                frame.partition,
                serde_json::to_string(&frame.surface_identity).unwrap_or_else(|_| "null".into()),
                frame.surface_ownership_confidence
            )
        }));
        content.push(json!({
            "type": "input_image",
            "image_url": format!("data:{mime};base64,{}", base64_encode(&bytes)),
            "detail": "high"
        }));
        image_count += 1;
        image_bytes += bytes.len();
        supplied_images.push(format!(
            "{}:{}:{}",
            frame.frame_id, frame.image_source_kind, frame.image_scope
        ));
        supplied_frame_ids.insert(frame.frame_id.clone());
        if let Some(hash) = &frame.local_image_handle_hash {
            image_hashes.push(hash.clone());
        }
    }
    if image_count == 0 {
        return Err(ModelFailureV1 {
            kind: ModelFailureKindV1::Unavailable,
            reason: "no_privacy_approved_readable_images".into(),
        });
    }
    // A keyframe is valid visual evidence only when its pixels were actually
    // transported. Structured elements, causal events, and deltas remain
    // independently referenceable even when a visual is skipped.
    evidence_catalog.retain(|_, reference| {
        reference.source_kind != "keyframe"
            || reference
                .frame_id
                .as_ref()
                .is_some_and(|frame_id| supplied_frame_ids.contains(frame_id))
    });
    let final_causal_keys = evidence_keys_for_source(&evidence_catalog, "causal_event");
    let final_delta_keys = evidence_keys_for_source(&evidence_catalog, "semantic_delta");
    let mut final_prior_keys = evidence_keys_for_source(&evidence_catalog, "prior_snapshot");
    final_prior_keys.extend(evidence_keys_for_source(
        &evidence_catalog,
        "prior_thread_revision",
    ));
    let final_task_keys = evidence_catalog
        .iter()
        .filter(|(_, reference)| {
            reference.source_kind == "causal_event"
                || packet
                    .canonical_elements
                    .iter()
                    .find(|element| element.element_id == reference.record_id)
                    .is_some_and(|element| {
                        element.task_eligible
                            && (element.authorship_status == AuthorshipStatusV2::User
                                || !element.causal_evidence_refs.is_empty())
                    })
        })
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    let final_object_keys = evidence_catalog
        .iter()
        .filter(|(_, reference)| {
            reference.source_kind == "canonical_element"
                && reference.content_hash.is_some()
                && packet
                    .canonical_elements
                    .iter()
                    .find(|element| element.element_id == reference.record_id)
                    .is_some_and(|element| element.task_eligible)
        })
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    let final_plan_keys = evidence_catalog
        .iter()
        .filter(|(_, reference)| {
            packet
                .canonical_elements
                .iter()
                .find(|element| element.element_id == reference.record_id)
                .is_some_and(|element| {
                    element.task_eligible && element.authorship_status == AuthorshipStatusV2::User
                })
        })
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    let evidence_policy = EvidenceKeyPolicyV1 {
        causal: final_causal_keys.iter().cloned().collect(),
        delta: final_delta_keys.iter().cloned().collect(),
        prior: final_prior_keys.iter().cloned().collect(),
        task_identity: final_task_keys.iter().cloned().collect(),
        task_object: final_object_keys.iter().cloned().collect(),
        user_plan: final_plan_keys.iter().cloned().collect(),
    };
    structured["evidence_reference_catalog"] =
        serde_json::to_value(&evidence_catalog).unwrap_or_else(|_| json!({}));
    let rules = &mut structured["request_policy"]["evidence_reference_rules"];
    rules["immediate_user_operation_requires_one_of"] = json!(final_causal_keys);
    rules["semantic_effect_requires_one_of"] = json!(final_delta_keys);
    rules["task_identity_requires_one_of"] = json!(final_task_keys);
    rules["task_object_also_requires_one_of"] = json!(final_object_keys);
    rules["continuity_or_return_requires_prior_plus_current"] = json!(final_prior_keys);
    rules["next_action_requires_user_authored_plan_key"] = json!(final_plan_keys);
    let structured_text = serde_json::to_string(&structured).map_err(|_| ModelFailureV1 {
        kind: ModelFailureKindV1::InvalidJson,
        reason: "structured_packet_serialization_failed".into(),
    })?;
    content.insert(0, json!({"type":"input_text", "text": structured_text}));
    let body = json!({
        "model": model,
        "store": false,
        "max_output_tokens": 6000,
        "text": {"format": {
            "type": "json_schema",
            "name": "smalltalk_task_truth_resolution",
            "strict": true,
            "schema": model_output_schema(
                packet,
                prior_threads,
                evidence_catalog.keys().cloned().collect(),
                &evidence_policy
            )
        }},
        "input": [
            {"role":"system", "content":[{"type":"input_text", "text": system_instruction()}]},
            {"role":"user", "content": content}
        ]
    });
    Ok(TaskTruthModelRequestV1 {
        body,
        audit: MultimodalRequestAuditV1 {
            request_schema: "smalltalk.task_truth_multimodal_request.v2".into(),
            provider: "openai".into(),
            model: model.into(),
            request_id,
            image_count,
            image_bytes,
            image_handle_hashes: image_hashes,
            skipped_images: skipped,
            structured_bytes: structured_text.len(),
            estimated_tokens: structured_text.len().div_ceil(4) + image_count * 1100,
            max_images: MAX_IMAGES,
            max_image_bytes: MAX_IMAGE_BYTES,
            privacy_exclusions,
            current_frame_readable_visual: packet.current_frame.model_eligible,
            current_frame_visual_reason: packet.current_frame.image_rejection_reason.clone(),
            supplied_images,
        },
        evidence_catalog,
        evidence_policy,
        prior_threads: prior_threads.to_vec(),
    })
}

fn system_instruction() -> &'static str {
    "You are Smalltalk's cloud multimodal semantic sensor. Reconstruct the chronological sequence, not a text recap: (1) what every selected screen is about, (2) which owned object or control the user interacted with, (3) what changed after that interaction, (4) which immediate activity the causal sequence supports, and (5) whether it continues, supports, verifies, interrupts, temporarily detours from, returns to, or replaces the earlier task. Return observed_surface, immediate_user_operation, semantic_effect_of_operation, current_subtask, likely_primary_task, task_object, relationship_to_prior, last_meaningful_progress, unfinished_state, and possible_next_action as separate fields. Evidence priority is: grounded user action plus resulting state change; repeated coherent work-object interaction; focus, committed typing, submit, navigation, app switch, or tool-output boundaries; foreground content; passive visible text; then chrome or historical text. Passive page vocabulary and window titles prove only observation. Browser/app chrome cannot establish the task. Distinguish user-authored input from application/agent output and third-party content. Prior task threads are bounded hypotheses, not truth. For continuation, supporting_research, verification, temporary_detour, interruption, or return_to_prior_task, copy one exact continuity_thread_id, continuity_thread_revision, and continuity_identity_token from prior_task_threads and cite both current evidence and the prior revision. For new_task or unrelated_or_unknown, continuity fields must be null. Set supersedes_thread_id only when current contradictory evidence explicitly supports replacing that exact prior thread; otherwise null. A prior snapshot cannot override newer contradictory evidence. When relatedness is weak, preserve an unrelated_or_unknown alternative. Return one to three hypotheses for resolved or ambiguous evidence, with field-level support, contradictions, and confidence. claim_evidence is keyed by semantic field: use an evidence object for every non-null field and for relationship_to_prior, and use null only when the corresponding semantic field is null. Explain null fields in missing_evidence. Evidence references must be opaque string keys copied exactly from evidence_reference_catalog. Never return record ids, frame ids, hashes, or evidence objects directly. Obey every field-specific key requirement in request_policy.evidence_reference_rules. Identity hashes, thread identities, and return anchors are opaque exact values constrained by the response schema; use null rather than descriptive prose. Give a next action only when the unfinished state and a user-authored plan key support it. Write semantic values as concise product-copy fragments, not analyst narration. Never refer to 'the user', and do not put uncertainty words such as 'likely', 'appears', or 'looks like' inside the task summary; confidence and alternative hypotheses carry uncertainty. likely_primary_task and current_subtask should be short activity phrases. last_meaningful_progress and unfinished_state should each contain one concrete sentence, not reasoning or a recap. Do not invent task objects, identifiers, intent, or actions. Return strict JSON only."
}

fn nullable_bounded_string(max_length: usize) -> Value {
    json!({"anyOf":[{"type":"null"},{"type":"string","maxLength":max_length}]})
}

fn nullable_exact_strings(values: Vec<String>) -> Value {
    let mut values = values
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    if values.is_empty() {
        json!({"type":"null"})
    } else {
        json!({"anyOf":[{"type":"null"},{"type":"string","enum":values}]})
    }
}

fn nullable_exact_i64(mut values: Vec<i64>) -> Value {
    values.sort_unstable();
    values.dedup();
    if values.is_empty() {
        json!({"type":"null"})
    } else {
        json!({"anyOf":[{"type":"null"},{"type":"integer","enum":values}]})
    }
}

fn nullable_enum(values: &[&str]) -> Value {
    json!({"anyOf":[{"type":"null"},{"type":"string","enum":values}]})
}

fn evidence_key_schema(mut evidence_keys: Vec<String>) -> Value {
    if evidence_keys.is_empty() {
        evidence_keys.push("no_evidence_available".to_string());
    }
    json!({"type":"string","enum":evidence_keys})
}

fn allowed_evidence_keys_for_field(
    field: &str,
    evidence_keys: &[String],
    evidence_policy: &EvidenceKeyPolicyV1,
) -> Vec<String> {
    let keys = match field {
        "immediate_user_operation" => evidence_policy.causal.clone(),
        "semantic_effect_of_operation" => evidence_policy.delta.clone(),
        "current_subtask" | "likely_primary_task" => evidence_policy.task_identity.clone(),
        "task_object" => evidence_policy
            .task_identity
            .intersection(&evidence_policy.task_object)
            .cloned()
            .collect(),
        "possible_next_action" => evidence_policy.user_plan.clone(),
        "relationship_to_prior" => evidence_policy
            .prior
            .iter()
            .cloned()
            .chain(evidence_keys.iter().cloned())
            .collect(),
        _ => evidence_keys.iter().cloned().collect(),
    };
    keys.into_iter().collect()
}

fn claim_evidence_schema(evidence_keys: &[String], evidence_policy: &EvidenceKeyPolicyV1) -> Value {
    let properties = MODEL_SEMANTIC_FIELDS
        .iter()
        .map(|field| {
            let allowed = allowed_evidence_keys_for_field(field, evidence_keys, evidence_policy);
            (
                (*field).to_string(),
                if allowed.is_empty() {
                    json!({"type":"null"})
                } else {
                    json!({"anyOf":[{
                        "type":"object", "additionalProperties":false,
                        "required":["claim","evidence_refs","confidence"],
                        "properties":{
                            "claim":{"type":"string","minLength":1},
                            "evidence_refs":{
                                "type":"array","minItems":1,"maxItems":8,
                                "items":evidence_key_schema(allowed)
                            },
                            "confidence":{"type":"number","minimum":0,"maximum":1}
                        }
                    }, {"type":"null"}]})
                },
            )
        })
        .collect::<serde_json::Map<_, _>>();
    json!({
        "type":"object",
        "additionalProperties":false,
        "required":MODEL_SEMANTIC_FIELDS,
        "properties":properties
    })
}

fn hypothesis_schema(
    packet: &ObservationPacketV2,
    prior_threads: &[PriorTaskThreadContextV1],
    evidence_keys: &[String],
    evidence_policy: &EvidenceKeyPolicyV1,
) -> Value {
    let app_identities = [
        packet.active_surface.app_name.clone(),
        packet.active_surface.app_bundle_id.clone(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let document_identities = [
        packet.active_surface.document_path_hash.clone(),
        packet.active_surface.browser_url_hash.clone(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let surface_identities = packet
        .active_surface
        .window_title_hash
        .clone()
        .into_iter()
        .collect::<Vec<_>>();
    let return_anchor_ids = packet
        .return_anchor_facts
        .iter()
        .map(|anchor| anchor.record_id.clone())
        .collect::<Vec<_>>();
    let continuity_thread_ids = prior_threads
        .iter()
        .map(|thread| thread.task_thread_id.clone())
        .collect::<Vec<_>>();
    let continuity_tokens = prior_threads
        .iter()
        .map(|thread| thread.identity_token.clone())
        .collect::<Vec<_>>();
    let continuity_revisions = prior_threads
        .iter()
        .map(|thread| thread.revision)
        .collect::<Vec<_>>();
    let task_identity = |max_length| {
        if evidence_policy.task_identity.is_empty() {
            json!({"type":"null"})
        } else {
            nullable_bounded_string(max_length)
        }
    };
    let task_object = if evidence_policy
        .task_identity
        .intersection(&evidence_policy.task_object)
        .next()
        .is_none()
    {
        json!({"type":"null"})
    } else {
        nullable_bounded_string(120)
    };
    let relationship_values = if evidence_policy.task_identity.is_empty() {
        vec!["unrelated_or_unknown"]
    } else if prior_threads.is_empty() {
        vec!["new_task", "unrelated_or_unknown"]
    } else {
        vec![
            "continuation",
            "supporting_research",
            "verification",
            "temporary_detour",
            "interruption",
            "new_task",
            "return_to_prior_task",
            "unrelated_or_unknown",
        ]
    };
    json!({
        "type":"object", "additionalProperties":false,
        "required":["hypothesis_id","observed_surface","immediate_user_operation","semantic_effect_of_operation","current_subtask","likely_primary_task","task_object","app_identity","surface_identity_hash","document_or_thread_identity_hash","execution_state","current_actor","waiting_on","last_meaningful_progress","unfinished_state","possible_next_action","relationship_to_prior","continuity_thread_id","continuity_thread_revision","continuity_identity_token","supersedes_thread_id","return_anchor_record_id","claim_evidence","contradictions","confidence_by_field","confidence"],
        "properties":{
            "hypothesis_id":{"type":"string"},
            "observed_surface":nullable_bounded_string(240),
            "immediate_user_operation":if evidence_policy.causal.is_empty() { json!({"type":"null"}) } else { nullable_bounded_string(180) },
            "semantic_effect_of_operation":if evidence_policy.delta.is_empty() { json!({"type":"null"}) } else { nullable_bounded_string(180) },
            "current_subtask":task_identity(140),
            "likely_primary_task":task_identity(140),
            "task_object":task_object, "app_identity":nullable_exact_strings(app_identities),
            "surface_identity_hash":nullable_exact_strings(surface_identities), "document_or_thread_identity_hash":nullable_exact_strings(document_identities),
            "execution_state":nullable_enum(&["active","composing","editing","reviewing","waiting","debugging","searching","comparing","blocked","interrupted","suspended","completed","superseded","idle_after_progress","unclear"]),
            "current_actor":nullable_enum(&["user","assistant_or_agent","application","unknown"]),
            "waiting_on":nullable_enum(&["user","assistant_or_agent","application","external","nothing","unknown"]),
            "last_meaningful_progress":nullable_bounded_string(220), "unfinished_state":nullable_bounded_string(220), "possible_next_action":if evidence_policy.user_plan.is_empty() { json!({"type":"null"}) } else { nullable_bounded_string(180) },
            "relationship_to_prior":{"type":"string","enum":relationship_values},
            "continuity_thread_id":nullable_exact_strings(continuity_thread_ids.clone()),
            "continuity_thread_revision":nullable_exact_i64(continuity_revisions),
            "continuity_identity_token":nullable_exact_strings(continuity_tokens),
            "supersedes_thread_id":nullable_exact_strings(continuity_thread_ids),
            "return_anchor_record_id":nullable_exact_strings(return_anchor_ids),
            "claim_evidence":claim_evidence_schema(evidence_keys, evidence_policy),
            "contradictions":{"type":"array","maxItems":12,"items":{
                "type":"object","additionalProperties":false,"required":["field","reason","evidence_refs"],
                "properties":{"field":{"type":"string"},"reason":{"type":"string"},"evidence_refs":{"type":"array","maxItems":8,"items":evidence_key_schema(evidence_keys.to_vec())}}
            }},
            "confidence_by_field":{
                "type":"object","additionalProperties":false,
                "required":["observed_surface","immediate_user_operation","semantic_effect_of_operation","current_subtask","likely_primary_task","task_object","app_identity","surface_identity_hash","document_or_thread_identity_hash","execution_state","current_actor","waiting_on","last_meaningful_progress","unfinished_state","possible_next_action","relationship_to_prior"],
                "properties":{
                    "observed_surface":{"type":"number","minimum":0,"maximum":1},
                    "immediate_user_operation":{"type":"number","minimum":0,"maximum":1},
                    "semantic_effect_of_operation":{"type":"number","minimum":0,"maximum":1},
                    "current_subtask":{"type":"number","minimum":0,"maximum":1},
                    "likely_primary_task":{"type":"number","minimum":0,"maximum":1},
                    "task_object":{"type":"number","minimum":0,"maximum":1},
                    "app_identity":{"type":"number","minimum":0,"maximum":1},
                    "surface_identity_hash":{"type":"number","minimum":0,"maximum":1},
                    "document_or_thread_identity_hash":{"type":"number","minimum":0,"maximum":1},
                    "execution_state":{"type":"number","minimum":0,"maximum":1},
                    "current_actor":{"type":"number","minimum":0,"maximum":1},
                    "waiting_on":{"type":"number","minimum":0,"maximum":1},
                    "last_meaningful_progress":{"type":"number","minimum":0,"maximum":1},
                    "unfinished_state":{"type":"number","minimum":0,"maximum":1},
                    "possible_next_action":{"type":"number","minimum":0,"maximum":1},
                    "relationship_to_prior":{"type":"number","minimum":0,"maximum":1}
                }
            },
            "confidence":{"type":"number","minimum":0,"maximum":1}
        }
    })
}

fn model_output_schema(
    packet: &ObservationPacketV2,
    prior_threads: &[PriorTaskThreadContextV1],
    evidence_keys: Vec<String>,
    evidence_policy: &EvidenceKeyPolicyV1,
) -> Value {
    json!({
        "type":"object","additionalProperties":false,
        "required":["schema","resolution_status","hypotheses","missing_evidence","policy_notes"],
        "properties":{
            "schema":{"type":"string","enum":[TASK_TRUTH_MODEL_OUTPUT_SCHEMA_V1]},
            "resolution_status":{"type":"string","enum":["resolved","ambiguous","insufficient_evidence","privacy_blocked","model_unavailable","provider_failure","invalid_response","verification_rejected"]},
            "hypotheses":{"type":"array","maxItems":3,"items":hypothesis_schema(packet, prior_threads, &evidence_keys, evidence_policy)},
            "missing_evidence":{"type":"array","maxItems":12,"items":{"type":"string"}},
            "policy_notes":{"type":"array","maxItems":12,"items":{"type":"string"}}
        }
    })
}

pub(crate) struct UnavailableModelClient {
    pub(crate) model: String,
    pub(crate) reason: String,
}

impl TaskTruthModelClient for UnavailableModelClient {
    fn provider_name(&self) -> &str {
        "openai"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn inference_origin(&self) -> InferenceOriginV1 {
        InferenceOriginV1::None
    }

    fn infer(
        &self,
        _: &TaskTruthModelRequestV1,
    ) -> Result<TaskTruthModelResponseV1, ModelFailureV1> {
        Err(ModelFailureV1 {
            kind: ModelFailureKindV1::Unavailable,
            reason: self.reason.clone(),
        })
    }
}

pub(crate) struct OpenAiTaskTruthModelClient {
    pub(crate) model: String,
    pub(crate) api_key: String,
    provider_attempts: std::sync::Mutex<Vec<ProviderAttemptMetadataV1>>,
}

impl OpenAiTaskTruthModelClient {
    pub(crate) fn new(model: String, api_key: String) -> Self {
        Self {
            model,
            api_key,
            provider_attempts: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl TaskTruthModelClient for OpenAiTaskTruthModelClient {
    fn provider_name(&self) -> &str {
        "openai"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn inference_origin(&self) -> InferenceOriginV1 {
        InferenceOriginV1::LiveCloud
    }

    fn provider_attempts(&self) -> Vec<ProviderAttemptMetadataV1> {
        self.provider_attempts
            .lock()
            .map(|attempts| attempts.clone())
            .unwrap_or_default()
    }

    fn infer(
        &self,
        request: &TaskTruthModelRequestV1,
    ) -> Result<TaskTruthModelResponseV1, ModelFailureV1> {
        if let Ok(mut attempts) = self.provider_attempts.lock() {
            attempts.clear();
        }
        let response =
            super::super::call_openai_responses_with_timeout(&self.api_key, &request.body, 90, 1)
                .map_err(classify_transport_failure)?;
        if let Ok(mut attempts) = self.provider_attempts.lock() {
            attempts.push(provider_attempt_metadata(&response));
        }
        let parse_and_validate = |response: &Value| {
            let parsed = parse_model_response(
                response,
                &request.evidence_catalog,
                Some(&request.evidence_policy),
            )?;
            validate_thread_links(&parsed.output, &request.prior_threads)?;
            Ok::<TaskTruthModelResponseV1, ModelFailureV1>(parsed)
        };
        let parsed = match parse_and_validate(&response) {
            Ok(parsed) => parsed,
            Err(first_failure) => {
                let Some(correction) = contract_correction(&first_failure, request, &response)
                else {
                    return Err(first_failure);
                };
                let correction_response = super::super::call_openai_responses_with_timeout(
                    &self.api_key,
                    &correction,
                    90,
                    0,
                )
                .map_err(classify_transport_failure)?;
                if let Ok(mut attempts) = self.provider_attempts.lock() {
                    attempts.push(provider_attempt_metadata(&correction_response));
                }
                parse_and_validate(&correction_response)?
            }
        };
        let provider_attempts = self.provider_attempts();
        Ok(TaskTruthModelResponseV1 {
            usage: aggregate_provider_usage(&provider_attempts),
            provider_attempts,
            ..parsed
        })
    }
}

pub(super) fn provider_attempt_metadata(response: &Value) -> ProviderAttemptMetadataV1 {
    let usage = response.get("usage");
    ProviderAttemptMetadataV1 {
        response_id: response
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string),
        request_id: response
            .get("request_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        model: response
            .get("model")
            .and_then(Value::as_str)
            .map(str::to_string),
        usage: ProviderUsageV1 {
            input_tokens: usage
                .and_then(|value| value.get("input_tokens"))
                .and_then(Value::as_i64),
            output_tokens: usage
                .and_then(|value| value.get("output_tokens"))
                .and_then(Value::as_i64),
            total_tokens: usage
                .and_then(|value| value.get("total_tokens"))
                .and_then(Value::as_i64),
        },
    }
}

pub(crate) fn aggregate_provider_usage(attempts: &[ProviderAttemptMetadataV1]) -> ProviderUsageV1 {
    let sum = |select: fn(&ProviderUsageV1) -> Option<i64>| {
        let values = attempts
            .iter()
            .filter_map(|attempt| select(&attempt.usage))
            .collect::<Vec<_>>();
        (!values.is_empty()).then(|| values.into_iter().sum())
    };
    ProviderUsageV1 {
        input_tokens: sum(|usage| usage.input_tokens),
        output_tokens: sum(|usage| usage.output_tokens),
        total_tokens: sum(|usage| usage.total_tokens),
    }
}

fn contract_correction(
    failure: &ModelFailureV1,
    request: &TaskTruthModelRequestV1,
    rejected_response: &Value,
) -> Option<Value> {
    let (field, reason) =
        if let Some(detail) = failure.reason.strip_prefix("evidence_policy_mismatch:") {
            detail.split_once(':')?
        } else {
            match failure.reason.as_str() {
                "partial_thread_continuity_identity"
                | "continuity_relationship_without_exact_thread_head"
                | "new_or_unrelated_task_reused_prior_thread"
                | "invalid_supersedes_thread_id"
                | "supersession_missing_prior_and_current_evidence"
                | "continuity_relationship_missing_prior_and_current_evidence" => {
                    ("relationship_to_prior", failure.reason.as_str())
                }
                _ => return None,
            }
        };
    let allowed_keys = allowed_evidence_keys_for_field(
        field,
        &request.evidence_catalog.keys().cloned().collect::<Vec<_>>(),
        &request.evidence_policy,
    );
    let rejected_output = structured_output_text(rejected_response)?;
    let mut body = request.body.clone();
    body.get_mut("input")?.as_array_mut()?.push(json!({
        "role":"user",
        "content":[{"type":"input_text","text":format!(
            "CORRECTION_REQUIRED. The rejected field is `{field}`. The exact failure is `{reason}`. Allowed evidence keys for this field are: {}. The previous rejected JSON was: {}. Return one corrected full response using only those keys for `{field}`. If none supports the field, set the semantic field and claim_evidence entry to null. Preserve every supported field that was not rejected. Do not invent a replacement task.",
            serde_json::to_string(&allowed_keys).unwrap_or_else(|_| "[]".into()),
            rejected_output,
        )}]
    }));
    Some(body)
}

fn structured_output_text(response: &Value) -> Option<String> {
    response
        .get("output_text")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            let chunks = response
                .get("output")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .flat_map(|item| {
                    item.get("content")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                })
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<String>();
            (!chunks.is_empty()).then_some(chunks)
        })
}

fn validate_thread_links(
    output: &TaskTruthModelOutputV1,
    prior_threads: &[PriorTaskThreadContextV1],
) -> Result<(), ModelFailureV1> {
    for hypothesis in &output.hypotheses {
        let linked = match (
            hypothesis.continuity_thread_id.as_deref(),
            hypothesis.continuity_thread_revision,
            hypothesis.continuity_identity_token.as_deref(),
        ) {
            (Some(thread_id), Some(revision), Some(token)) => prior_threads.iter().find(|thread| {
                thread.task_thread_id == thread_id
                    && thread.revision == revision
                    && thread.identity_token == token
            }),
            (None, None, None) => None,
            _ => {
                return Err(ModelFailureV1 {
                    kind: ModelFailureKindV1::InvalidJson,
                    reason: "partial_thread_continuity_identity".into(),
                })
            }
        };
        let continuity_required = matches!(
            hypothesis.relationship_to_prior,
            TaskRelationshipV1::Continuation
                | TaskRelationshipV1::SupportingResearch
                | TaskRelationshipV1::Verification
                | TaskRelationshipV1::TemporaryDetour
                | TaskRelationshipV1::Interruption
                | TaskRelationshipV1::ReturnToPriorTask
        );
        if continuity_required && linked.is_none() {
            return Err(ModelFailureV1 {
                kind: ModelFailureKindV1::InvalidJson,
                reason: "continuity_relationship_without_exact_thread_head".into(),
            });
        }
        if matches!(
            hypothesis.relationship_to_prior,
            TaskRelationshipV1::NewTask | TaskRelationshipV1::UnrelatedOrUnknown
        ) && linked.is_some()
        {
            return Err(ModelFailureV1 {
                kind: ModelFailureKindV1::InvalidJson,
                reason: "new_or_unrelated_task_reused_prior_thread".into(),
            });
        }
        if let Some(supersedes) = hypothesis.supersedes_thread_id.as_deref() {
            let superseded = prior_threads
                .iter()
                .find(|thread| thread.task_thread_id == supersedes);
            if hypothesis.relationship_to_prior != TaskRelationshipV1::NewTask
                || superseded.is_none()
            {
                return Err(ModelFailureV1 {
                    kind: ModelFailureKindV1::InvalidJson,
                    reason: "invalid_supersedes_thread_id".into(),
                });
            }
            let superseded = superseded.expect("superseded thread validated");
            let refs = hypothesis
                .claim_evidence
                .get("relationship_to_prior")
                .and_then(Option::as_ref)
                .map(|claim| claim.evidence_refs.as_slice())
                .unwrap_or(&[]);
            let expected_prior_record =
                format!("{}:{}", superseded.task_thread_id, superseded.revision);
            let has_prior = refs.iter().any(|reference| {
                reference.source_kind == "prior_thread_revision"
                    && reference.record_id == expected_prior_record
            });
            let has_current = refs.iter().any(|reference| {
                !matches!(
                    reference.source_kind.as_str(),
                    "prior_snapshot" | "prior_thread_revision"
                )
            });
            if !has_prior || !has_current {
                return Err(ModelFailureV1 {
                    kind: ModelFailureKindV1::InvalidJson,
                    reason: "supersession_missing_prior_and_current_evidence".into(),
                });
            }
        }
        if let Some(thread) = linked {
            let refs = hypothesis
                .claim_evidence
                .get("relationship_to_prior")
                .and_then(Option::as_ref)
                .map(|claim| claim.evidence_refs.as_slice())
                .unwrap_or(&[]);
            let expected_prior_record = format!("{}:{}", thread.task_thread_id, thread.revision);
            let has_prior = refs.iter().any(|reference| {
                reference.source_kind == "prior_thread_revision"
                    && reference.record_id == expected_prior_record
            });
            let has_current = refs.iter().any(|reference| {
                !matches!(
                    reference.source_kind.as_str(),
                    "prior_snapshot" | "prior_thread_revision"
                )
            });
            if !has_prior || !has_current {
                return Err(ModelFailureV1 {
                    kind: ModelFailureKindV1::InvalidJson,
                    reason: "continuity_relationship_missing_prior_and_current_evidence".into(),
                });
            }
        }
    }
    Ok(())
}

fn classify_transport_failure(error: String) -> ModelFailureV1 {
    let lower = error.to_ascii_lowercase();
    let kind = if lower.contains("timeout") || lower.contains("timed out") {
        ModelFailureKindV1::Timeout
    } else if lower.contains("400") || lower.contains("invalid_request") {
        ModelFailureKindV1::RequestInvalid
    } else if lower.contains("401")
        || lower.contains("403")
        || lower.contains("authentication")
        || lower.contains("permission")
    {
        ModelFailureKindV1::RequestRejected
    } else if lower.contains("404") || lower.contains("model_not_found") {
        ModelFailureKindV1::Unavailable
    } else {
        ModelFailureKindV1::ProviderError
    };
    ModelFailureV1 {
        kind,
        reason: "provider_request_failed".into(),
    }
}

fn expand_evidence_key_array(
    value: &mut Value,
    catalog: &BTreeMap<String, EvidenceHandleV2>,
) -> Result<(), ModelFailureV1> {
    let Some(items) = value.as_array_mut() else {
        return Err(ModelFailureV1 {
            kind: ModelFailureKindV1::InvalidJson,
            reason: "evidence_refs_not_array".into(),
        });
    };
    for item in items {
        if item.is_object() && catalog.is_empty() {
            // Deterministic fixture compatibility. Live requests always carry a
            // catalog and their strict schema permits opaque keys only.
            continue;
        }
        let key = item.as_str().ok_or_else(|| ModelFailureV1 {
            kind: ModelFailureKindV1::InvalidJson,
            reason: "evidence_reference_must_be_catalog_key".into(),
        })?;
        let reference = catalog.get(key).ok_or_else(|| ModelFailureV1 {
            kind: ModelFailureKindV1::InvalidJson,
            reason: "evidence_catalog_key_not_in_request".into(),
        })?;
        *item = serde_json::to_value(reference).map_err(|_| ModelFailureV1 {
            kind: ModelFailureKindV1::InvalidJson,
            reason: "evidence_catalog_expansion_failed".into(),
        })?;
    }
    Ok(())
}

fn expand_model_evidence_keys(
    output: &mut Value,
    catalog: &BTreeMap<String, EvidenceHandleV2>,
) -> Result<(), ModelFailureV1> {
    let Some(hypotheses) = output.get_mut("hypotheses").and_then(Value::as_array_mut) else {
        return Ok(());
    };
    for hypothesis in hypotheses {
        if let Some(claims) = hypothesis
            .get_mut("claim_evidence")
            .and_then(Value::as_object_mut)
        {
            for claim in claims.values_mut().filter(|claim| !claim.is_null()) {
                if let Some(references) = claim.get_mut("evidence_refs") {
                    expand_evidence_key_array(references, catalog)?;
                }
            }
        }
        if let Some(contradictions) = hypothesis
            .get_mut("contradictions")
            .and_then(Value::as_array_mut)
        {
            for contradiction in contradictions {
                if let Some(references) = contradiction.get_mut("evidence_refs") {
                    expand_evidence_key_array(references, catalog)?;
                }
            }
        }
    }
    Ok(())
}

fn validate_opaque_evidence_policy(
    output: &Value,
    policy: &EvidenceKeyPolicyV1,
) -> Result<(), String> {
    let Some(hypotheses) = output.get("hypotheses").and_then(Value::as_array) else {
        return Ok(());
    };
    for hypothesis in hypotheses {
        let claims = hypothesis.get("claim_evidence").and_then(Value::as_object);
        let claim_keys = |field: &str| -> Vec<&str> {
            claims
                .and_then(|claims| claims.get(field))
                .filter(|claim| !claim.is_null())
                .and_then(|claim| claim.get("evidence_refs"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect()
        };
        let requires_one = |field: &str, allowed: &BTreeSet<String>| {
            let keys = claim_keys(field);
            keys.is_empty() || keys.iter().any(|key| allowed.contains(*key))
        };
        if !requires_one("immediate_user_operation", &policy.causal) {
            return Err("immediate_user_operation:immediate_operation_without_causal_key".into());
        }
        if !requires_one("semantic_effect_of_operation", &policy.delta) {
            return Err("semantic_effect_of_operation:semantic_effect_without_delta_key".into());
        }
        for field in ["current_subtask", "likely_primary_task", "task_object"] {
            if !requires_one(field, &policy.task_identity) {
                return Err(format!(
                    "{field}:task_identity_without_user_authored_or_causal_key"
                ));
            }
        }
        if !requires_one("task_object", &policy.task_object) {
            return Err("task_object:task_object_without_eligible_hashed_object_key".into());
        }
        if !requires_one("possible_next_action", &policy.user_plan) {
            return Err("possible_next_action:next_action_without_user_plan_key".into());
        }
        let relationship_keys = claim_keys("relationship_to_prior");
        if !relationship_keys.is_empty()
            && !policy.prior.is_empty()
            && (!relationship_keys
                .iter()
                .any(|key| policy.prior.contains(*key))
                || !relationship_keys
                    .iter()
                    .any(|key| !policy.prior.contains(*key)))
        {
            return Err("relationship_to_prior:relationship_without_prior_and_current_keys".into());
        }
    }
    Ok(())
}

fn parse_model_response(
    response: &Value,
    evidence_catalog: &BTreeMap<String, EvidenceHandleV2>,
    evidence_policy: Option<&EvidenceKeyPolicyV1>,
) -> Result<TaskTruthModelResponseV1, ModelFailureV1> {
    if response.get("status").and_then(Value::as_str) == Some("incomplete") {
        return Err(ModelFailureV1 {
            kind: ModelFailureKindV1::ProviderError,
            reason: "incomplete_response".into(),
        });
    }
    let mut refusal = false;
    let text = response
        .get("output_text")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            let mut chunks = Vec::new();
            for part in response
                .get("output")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .flat_map(|item| {
                    item.get("content")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                })
            {
                if part.get("type").and_then(Value::as_str) == Some("refusal") {
                    refusal = true;
                }
                if let Some(text) = part.get("text").and_then(Value::as_str) {
                    chunks.push(text.to_string());
                }
            }
            (!chunks.is_empty()).then(|| chunks.join(""))
        });
    if refusal {
        return Err(ModelFailureV1 {
            kind: ModelFailureKindV1::PolicyRefusal,
            reason: "provider_policy_refusal".into(),
        });
    }
    let text = text.ok_or_else(|| ModelFailureV1 {
        kind: ModelFailureKindV1::InvalidJson,
        reason: "missing_structured_output".into(),
    })?;
    let mut output_value: Value = serde_json::from_str(&text).map_err(|_| ModelFailureV1 {
        kind: ModelFailureKindV1::InvalidJson,
        reason: "invalid_structured_output".into(),
    })?;
    if let Some(policy) = evidence_policy {
        loop {
            let Err(reason) = validate_opaque_evidence_policy(&output_value, policy) else {
                break;
            };
            let field = reason.split(':').next().unwrap_or_default();
            if matches!(
                field,
                "immediate_user_operation"
                    | "semantic_effect_of_operation"
                    | "task_object"
                    | "possible_next_action"
            ) {
                null_unsupported_optional_field(&mut output_value, field);
            } else {
                return Err(ModelFailureV1 {
                    kind: ModelFailureKindV1::InvalidJson,
                    reason: format!("evidence_policy_mismatch:{reason}"),
                });
            }
        }
    }
    expand_model_evidence_keys(&mut output_value, evidence_catalog)?;
    let output: TaskTruthModelOutputV1 =
        serde_json::from_value(output_value).map_err(|_| ModelFailureV1 {
            kind: ModelFailureKindV1::InvalidJson,
            reason: "invalid_structured_output".into(),
        })?;
    let requires_hypothesis = matches!(
        output.resolution_status,
        ResolutionStatusV1::Resolved | ResolutionStatusV1::Ambiguous
    );
    if output.schema != TASK_TRUTH_MODEL_OUTPUT_SCHEMA_V1 {
        return Err(ModelFailureV1 {
            kind: ModelFailureKindV1::InvalidJson,
            reason: "schema_contract_mismatch:version".into(),
        });
    }
    if output.hypotheses.len() > 3 {
        return Err(ModelFailureV1 {
            kind: ModelFailureKindV1::InvalidJson,
            reason: "schema_contract_mismatch:hypothesis_count_exceeds_three".into(),
        });
    }
    if requires_hypothesis && output.hypotheses.is_empty() {
        return Err(ModelFailureV1 {
            kind: ModelFailureKindV1::InvalidJson,
            reason: "schema_contract_mismatch:resolved_without_hypothesis".into(),
        });
    }
    if let Err(reason) = validate_model_output(&output) {
        return Err(ModelFailureV1 {
            kind: ModelFailureKindV1::InvalidJson,
            reason: format!("schema_contract_mismatch:{reason}"),
        });
    }
    let metadata = provider_attempt_metadata(response);
    Ok(TaskTruthModelResponseV1 {
        output,
        provider_response_id: metadata.response_id.clone(),
        provider_request_id: metadata.request_id.clone(),
        provider_model: metadata.model.clone(),
        usage: metadata.usage.clone(),
        provider_attempts: vec![metadata],
    })
}

fn null_unsupported_optional_field(output: &mut Value, field: &str) {
    let Some(hypotheses) = output.get_mut("hypotheses").and_then(Value::as_array_mut) else {
        return;
    };
    for hypothesis in hypotheses {
        if let Some(object) = hypothesis.as_object_mut() {
            object.insert(field.to_string(), Value::Null);
            if let Some(claims) = object
                .get_mut("claim_evidence")
                .and_then(Value::as_object_mut)
            {
                claims.insert(field.to_string(), Value::Null);
            }
            if let Some(confidence) = object
                .get_mut("confidence_by_field")
                .and_then(Value::as_object_mut)
            {
                confidence.insert(field.to_string(), json!(0.0));
            }
        }
    }
    if let Some(notes) = output.get_mut("policy_notes").and_then(Value::as_array_mut) {
        if notes.len() < 12 {
            notes.push(json!(format!("unsupported_optional_field_removed:{field}")));
        }
    }
}

fn validate_model_output(output: &TaskTruthModelOutputV1) -> Result<(), &'static str> {
    let expected_fields = MODEL_SEMANTIC_FIELDS
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut ids = std::collections::BTreeSet::new();
    for hypothesis in &output.hypotheses {
        if hypothesis.hypothesis_id.trim().is_empty() {
            return Err("empty_hypothesis_id");
        }
        if !ids.insert(hypothesis.hypothesis_id.as_str()) {
            return Err("duplicate_hypothesis_id");
        }
        if !hypothesis.confidence.is_finite() || !(0.0..=1.0).contains(&hypothesis.confidence) {
            return Err("invalid_overall_confidence");
        }
        if hypothesis
            .confidence_by_field
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>()
            != expected_fields
        {
            return Err("confidence_field_set_mismatch");
        }
        if !hypothesis
            .confidence_by_field
            .values()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
        {
            return Err("invalid_field_confidence");
        }
        if hypothesis
            .claim_evidence
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>()
            != expected_fields
        {
            return Err("claim_evidence_field_set_mismatch");
        }
        for (field, claim) in &hypothesis.claim_evidence {
            let Some(claim) = claim.as_ref() else {
                if hypothesis_field_has_value(hypothesis, field) {
                    return Err("non_null_field_without_claim_evidence");
                }
                continue;
            };
            if claim.claim.trim().is_empty() {
                return Err("empty_claim");
            }
            if claim.evidence_refs.is_empty() {
                return Err("claim_without_evidence");
            }
            if !claim.confidence.is_finite() || !(0.0..=1.0).contains(&claim.confidence) {
                return Err("invalid_claim_confidence");
            }
            if !claim.evidence_refs.iter().all(|reference| {
                matches!(
                    reference.source_kind.as_str(),
                    "canonical_element"
                        | "causal_event"
                        | "keyframe"
                        | "semantic_delta"
                        | "transition"
                        | "capture_trigger"
                        | "return_anchor_fact"
                        | "prior_snapshot"
                        | "prior_thread_revision"
                ) && !reference.record_id.trim().is_empty()
            }) {
                return Err("invalid_evidence_reference");
            }
        }
        if !hypothesis
            .contradictions
            .iter()
            .all(|item| expected_fields.contains(item.field.as_str()))
        {
            return Err("unknown_contradiction_field");
        }
    }
    Ok(())
}

fn hypothesis_field_has_value(hypothesis: &ModelTaskHypothesisV1, field: &str) -> bool {
    match field {
        "observed_surface" => hypothesis.observed_surface.is_some(),
        "immediate_user_operation" => hypothesis.immediate_user_operation.is_some(),
        "semantic_effect_of_operation" => hypothesis.semantic_effect_of_operation.is_some(),
        "current_subtask" => hypothesis.current_subtask.is_some(),
        "likely_primary_task" => hypothesis.likely_primary_task.is_some(),
        "task_object" => hypothesis.task_object.is_some(),
        "app_identity" => hypothesis.app_identity.is_some(),
        "surface_identity_hash" => hypothesis.surface_identity_hash.is_some(),
        "document_or_thread_identity_hash" => hypothesis.document_or_thread_identity_hash.is_some(),
        "execution_state" => hypothesis.execution_state.is_some(),
        "current_actor" => hypothesis.current_actor.is_some(),
        "waiting_on" => hypothesis.waiting_on.is_some(),
        "last_meaningful_progress" => hypothesis.last_meaningful_progress.is_some(),
        "unfinished_state" => hypothesis.unfinished_state.is_some(),
        "possible_next_action" => hypothesis.possible_next_action.is_some(),
        "relationship_to_prior" => true,
        _ => false,
    }
}

#[cfg(test)]
pub(crate) struct FixtureModelClient {
    pub(crate) model: String,
    pub(crate) responses: std::sync::Mutex<Vec<Result<TaskTruthModelResponseV1, ModelFailureV1>>>,
}

#[cfg(test)]
impl TaskTruthModelClient for FixtureModelClient {
    fn provider_name(&self) -> &str {
        "fixture"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn inference_origin(&self) -> InferenceOriginV1 {
        InferenceOriginV1::Fixture
    }

    fn infer(
        &self,
        _: &TaskTruthModelRequestV1,
    ) -> Result<TaskTruthModelResponseV1, ModelFailureV1> {
        self.responses.lock().unwrap().remove(0)
    }
}

pub(crate) fn is_control_role(role: RegionRoleV2) -> bool {
    matches!(
        role,
        RegionRoleV2::Navigation
            | RegionRoleV2::Toolbar
            | RegionRoleV2::Control
            | RegionRoleV2::Status
            | RegionRoleV2::Notification
            | RegionRoleV2::Sidebar
            | RegionRoleV2::BrowserChrome
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuation::task_truth_v2::observation_packet::{
        ActiveSurfaceIdentityV2, KeyframeReferenceV2, PacketSizeAccountingV2,
    };
    use std::collections::BTreeMap;

    struct PreservedFailureClient {
        attempts: Vec<ProviderAttemptMetadataV1>,
    }

    impl TaskTruthModelClient for PreservedFailureClient {
        fn provider_name(&self) -> &str {
            "openai"
        }

        fn model_name(&self) -> &str {
            "fixture-model"
        }

        fn inference_origin(&self) -> InferenceOriginV1 {
            InferenceOriginV1::LiveCloud
        }

        fn provider_attempts(&self) -> Vec<ProviderAttemptMetadataV1> {
            self.attempts.clone()
        }

        fn infer(
            &self,
            _: &TaskTruthModelRequestV1,
        ) -> Result<TaskTruthModelResponseV1, ModelFailureV1> {
            Err(ModelFailureV1 {
                kind: ModelFailureKindV1::InvalidJson,
                reason: "evidence_policy_mismatch:current_subtask:task_identity_without_user_authored_or_causal_key".into(),
            })
        }
    }

    fn valid_hypothesis(id: &str) -> ModelTaskHypothesisV1 {
        let evidence = EvidenceHandleV2 {
            source_kind: "keyframe".into(),
            record_id: "frame-current".into(),
            frame_id: Some("frame-current".into()),
            content_hash: None,
        };
        ModelTaskHypothesisV1 {
            hypothesis_id: id.into(),
            observed_surface: Some("A code editor".into()),
            immediate_user_operation: Some("Edited a file".into()),
            semantic_effect_of_operation: Some("The file changed".into()),
            current_subtask: Some("Implement the resolver".into()),
            likely_primary_task: Some("Implement cloud task inference".into()),
            task_object: Some("Task Truth resolver".into()),
            app_identity: Some("Test".into()),
            surface_identity_hash: None,
            document_or_thread_identity_hash: None,
            execution_state: Some("editing".into()),
            current_actor: Some("user".into()),
            waiting_on: Some("nothing".into()),
            last_meaningful_progress: Some("Updated the schema".into()),
            unfinished_state: Some("Verifier tests remain".into()),
            possible_next_action: Some("Run the verifier tests".into()),
            relationship_to_prior: TaskRelationshipV1::Continuation,
            continuity_thread_id: None,
            continuity_thread_revision: None,
            continuity_identity_token: None,
            supersedes_thread_id: None,
            return_anchor_record_id: None,
            claim_evidence: MODEL_SEMANTIC_FIELDS
                .iter()
                .map(|field| {
                    (
                        (*field).into(),
                        Some(ModelClaimEvidenceV1 {
                            claim: format!("supported {field}"),
                            evidence_refs: vec![evidence.clone()],
                            confidence: 0.8,
                        }),
                    )
                })
                .collect(),
            contradictions: vec![FieldContradictionV1 {
                field: "relationship_to_prior".into(),
                reason: "A new-task interpretation remains possible".into(),
                evidence_refs: vec![evidence],
            }],
            confidence_by_field: MODEL_SEMANTIC_FIELDS
                .iter()
                .map(|field| ((*field).into(), 0.8))
                .collect(),
            confidence: 0.8,
        }
    }

    fn valid_output(hypotheses: Vec<ModelTaskHypothesisV1>) -> TaskTruthModelOutputV1 {
        TaskTruthModelOutputV1 {
            schema: TASK_TRUTH_MODEL_OUTPUT_SCHEMA_V1.into(),
            resolution_status: if hypotheses.len() > 1 {
                ResolutionStatusV1::Ambiguous
            } else {
                ResolutionStatusV1::Resolved
            },
            hypotheses,
            missing_evidence: Vec::new(),
            policy_notes: Vec::new(),
        }
    }

    fn packet(path: Option<String>, private: bool) -> ObservationPacketV2 {
        let frame = KeyframeReferenceV2 {
            frame_id: "frame-current".into(),
            observed_at_ms: 1_000,
            partition: EvidencePartitionV2::Current,
            surface_identity: ActiveSurfaceIdentityV2 {
                app_name: Some("Test".into()),
                app_bundle_id: Some("com.test".into()),
                window_title_hash: None,
                window_id: Some(1),
                browser_url_hash: None,
                document_path_hash: None,
            },
            surface_ownership_confidence: 0.95,
            privacy_status: if private { "private" } else { "allowed" }.into(),
            model_eligible: !private,
            image_source_kind: "native_active_window".into(),
            image_scope: "active_window".into(),
            image_width: Some(100),
            image_height: Some(100),
            image_rejection_reason: private.then(|| "privacy_blocked".into()),
            crop_pixels: None,
            local_image_handle_hash: Some("durable-image-hash".into()),
            ephemeral_local_image_path: path,
            selection_reasons: vec!["manual_continue_boundary".into()],
        };
        ObservationPacketV2 {
            schema: "smalltalk.observation_packet.v2".into(),
            packet_id: "packet-model-test".into(),
            observed_at_ms: 1_000,
            session_id: Some("session-test".into()),
            evidence_watermark: "watermark".into(),
            active_surface: ActiveSurfaceIdentityV2 {
                app_name: Some("Test".into()),
                app_bundle_id: Some("com.test".into()),
                window_title_hash: None,
                window_id: Some(1),
                browser_url_hash: None,
                document_path_hash: None,
            },
            current_frame: frame.clone(),
            semantic_keyframes: vec![frame],
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
            evidence_quality: "pixels_plus_events".into(),
            missing_source_notes: Vec::new(),
            conflicting_observations: Vec::new(),
            partitions: BTreeMap::from([(
                EvidencePartitionV2::Current,
                vec!["frame-current".into()],
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

    fn image_file() -> std::path::PathBuf {
        static NEXT_IMAGE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let path = std::env::temp_dir().join(format!(
            "smalltalk-task-truth-model-{}-{}.png",
            std::process::id(),
            NEXT_IMAGE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        // The transport only needs real image bytes here; provider decoding is not used in tests.
        fs::write(&path, b"\x89PNG\r\n\x1a\nfixture").unwrap();
        path
    }

    #[test]
    fn request_contains_real_image_input_but_serialized_packet_omits_local_path() {
        let path = image_file();
        let packet = packet(Some(path.to_string_lossy().into_owned()), false);
        let request = build_multimodal_request(&packet, None, &[], "fixture-model", None).unwrap();
        assert_eq!(request.audit.image_count, 1);
        assert_eq!(
            request.audit.image_handle_hashes,
            vec!["durable-image-hash"]
        );
        let body = request.body.to_string();
        assert!(body.contains("data:image/png;base64,"));
        assert!(body.contains("evidence_reference_rules"));
        assert!(body.contains("evidence_reference_catalog"));
        assert!(body.contains("evidence_0001"));
        assert_eq!(
            request
                .evidence_catalog
                .get("evidence_0001")
                .map(|reference| reference.record_id.as_str()),
            Some("frame-current")
        );
        assert!(!body.contains(path.to_string_lossy().as_ref()));
        assert!(!serde_json::to_string(&packet)
            .unwrap()
            .contains(path.to_string_lossy().as_ref()));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn skipped_visuals_are_not_exposed_as_citable_keyframes() {
        let path = image_file();
        let mut packet = packet(Some(path.to_string_lossy().into_owned()), false);
        let mut background = packet.current_frame.clone();
        background.frame_id = "frame-background".into();
        background.partition = EvidencePartitionV2::Background;
        background.selection_reasons = vec!["background_context".into()];
        packet.semantic_keyframes.push(background);

        let request = build_multimodal_request(&packet, None, &[], "fixture-model", None).unwrap();

        assert_eq!(request.audit.image_count, 1);
        assert!(request
            .audit
            .privacy_exclusions
            .iter()
            .any(|reason| reason == "frame-background:background_display"));
        assert!(request
            .evidence_catalog
            .values()
            .any(|reference| reference.record_id == "frame-current"));
        assert!(!request
            .evidence_catalog
            .values()
            .any(|reference| reference.record_id == "frame-background"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn opaque_evidence_keys_expand_to_exact_packet_handles() {
        let path = image_file();
        let packet = packet(Some(path.to_string_lossy().into_owned()), false);
        let request = build_multimodal_request(&packet, None, &[], "fixture-model", None).unwrap();
        let mut value = serde_json::to_value(valid_output(vec![valid_hypothesis("h1")])).unwrap();
        for hypothesis in value["hypotheses"].as_array_mut().unwrap() {
            for claim in hypothesis["claim_evidence"]
                .as_object_mut()
                .unwrap()
                .values_mut()
                .filter(|claim| !claim.is_null())
            {
                claim["evidence_refs"] = json!(["evidence_0001"]);
            }
            for contradiction in hypothesis["contradictions"].as_array_mut().unwrap() {
                contradiction["evidence_refs"] = json!(["evidence_0001"]);
            }
        }
        let parsed = parse_model_response(
            &json!({"output_text": value.to_string()}),
            &request.evidence_catalog,
            None,
        )
        .unwrap();
        let reference = &parsed.output.hypotheses[0]
            .claim_evidence
            .get("observed_surface")
            .and_then(Option::as_ref)
            .unwrap()
            .evidence_refs[0];
        assert_eq!(reference.source_kind, "keyframe");
        assert_eq!(reference.record_id, "frame-current");
        assert_eq!(reference.frame_id.as_deref(), Some("frame-current"));
        assert_eq!(
            reference.content_hash.as_deref(),
            Some("durable-image-hash")
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn response_schema_constrains_lifecycle_identity_and_unavailable_anchor_values() {
        let path = image_file();
        let packet = packet(Some(path.to_string_lossy().into_owned()), false);
        let request = build_multimodal_request(&packet, None, &[], "fixture-model", None).unwrap();
        let schema = &request.body["text"]["format"]["schema"];
        let hypothesis = &schema["properties"]["hypotheses"]["items"]["properties"];
        assert!(hypothesis["execution_state"]
            .to_string()
            .contains("idle_after_progress"));
        assert!(hypothesis["current_actor"]
            .to_string()
            .contains("assistant_or_agent"));
        assert!(hypothesis["waiting_on"].to_string().contains("nothing"));
        assert!(hypothesis["app_identity"].to_string().contains("com.test"));
        assert_eq!(hypothesis["surface_identity_hash"]["type"], "null");
        assert_eq!(
            hypothesis["document_or_thread_identity_hash"]["type"],
            "null"
        );
        assert_eq!(hypothesis["return_anchor_record_id"]["type"], "null");
        for field in [
            "immediate_user_operation",
            "semantic_effect_of_operation",
            "current_subtask",
            "likely_primary_task",
            "task_object",
            "possible_next_action",
        ] {
            assert_eq!(
                hypothesis[field]["type"], "null",
                "unsupported field {field} must be schema-constrained to null"
            );
            assert_eq!(
                hypothesis["claim_evidence"]["properties"][field]["type"], "null",
                "unsupported claim {field} must also be schema-constrained to null"
            );
        }
        assert_eq!(
            hypothesis["relationship_to_prior"]["enum"],
            json!(["unrelated_or_unknown"])
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn supersession_requires_exact_prior_revision_and_current_evidence() {
        let prior = PriorTaskThreadContextV1 {
            task_thread_id: "thread-prior".into(),
            identity_token: "identity-prior".into(),
            revision: 4,
            status: "active".into(),
            current_session_id: "session-a".into(),
            session_lineage: vec!["session-a".into()],
            head_snapshot_id: "snapshot-prior".into(),
            task_summary: Some("Prior task".into()),
            task_object: Some("prior-object".into()),
            execution_state: "active".into(),
            last_meaningful_progress: Some("Prior progress".into()),
            unfinished_state: Some("Prior work remains".into()),
            last_supported_at_ms: 900,
        };
        let mut hypothesis = valid_hypothesis("hypothesis-new");
        hypothesis.relationship_to_prior = TaskRelationshipV1::NewTask;
        hypothesis.continuity_thread_id = None;
        hypothesis.continuity_thread_revision = None;
        hypothesis.continuity_identity_token = None;
        hypothesis.supersedes_thread_id = Some(prior.task_thread_id.clone());
        let output = valid_output(vec![hypothesis.clone()]);
        assert_eq!(
            validate_thread_links(&output, std::slice::from_ref(&prior))
                .unwrap_err()
                .reason,
            "supersession_missing_prior_and_current_evidence"
        );

        let relationship = hypothesis
            .claim_evidence
            .get_mut("relationship_to_prior")
            .and_then(Option::as_mut)
            .unwrap();
        relationship.evidence_refs.push(EvidenceHandleV2 {
            source_kind: "prior_thread_revision".into(),
            record_id: "thread-prior:4".into(),
            frame_id: None,
            content_hash: Some("identity-prior".into()),
        });
        validate_thread_links(
            &valid_output(vec![hypothesis]),
            std::slice::from_ref(&prior),
        )
        .unwrap();
    }

    #[test]
    fn unknown_opaque_evidence_key_is_rejected_before_verification() {
        let path = image_file();
        let packet = packet(Some(path.to_string_lossy().into_owned()), false);
        let request = build_multimodal_request(&packet, None, &[], "fixture-model", None).unwrap();
        let mut value = serde_json::to_value(valid_output(vec![valid_hypothesis("h1")])).unwrap();
        for claim in value["hypotheses"][0]["claim_evidence"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .filter(|claim| !claim.is_null())
        {
            claim["evidence_refs"] = json!(["evidence_0001"]);
        }
        for contradiction in value["hypotheses"][0]["contradictions"]
            .as_array_mut()
            .unwrap()
        {
            contradiction["evidence_refs"] = json!(["evidence_0001"]);
        }
        value["hypotheses"][0]["claim_evidence"]["observed_surface"]["evidence_refs"] =
            json!(["evidence_not_in_request"]);
        let error = parse_model_response(
            &json!({"output_text": value.to_string()}),
            &request.evidence_catalog,
            None,
        )
        .unwrap_err();
        assert_eq!(error.reason, "evidence_catalog_key_not_in_request");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn field_specific_evidence_policy_rejects_keyframe_only_task_claims() {
        let path = image_file();
        let packet = packet(Some(path.to_string_lossy().into_owned()), false);
        let request = build_multimodal_request(&packet, None, &[], "fixture-model", None).unwrap();
        let mut value = serde_json::to_value(valid_output(vec![valid_hypothesis("h1")])).unwrap();
        for claim in value["hypotheses"][0]["claim_evidence"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .filter(|claim| !claim.is_null())
        {
            claim["evidence_refs"] = json!(["evidence_0001"]);
        }
        for contradiction in value["hypotheses"][0]["contradictions"]
            .as_array_mut()
            .unwrap()
        {
            contradiction["evidence_refs"] = json!(["evidence_0001"]);
        }

        let error = parse_model_response(
            &json!({"output_text": value.to_string()}),
            &request.evidence_catalog,
            Some(&request.evidence_policy),
        )
        .unwrap_err();

        assert_eq!(
            error.reason,
            "evidence_policy_mismatch:current_subtask:task_identity_without_user_authored_or_causal_key"
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn optional_bad_evidence_is_removed_without_losing_supported_task_identity() {
        let mut output = valid_output(vec![valid_hypothesis("h1")]);
        let hypothesis = &mut output.hypotheses[0];
        hypothesis.immediate_user_operation = None;
        hypothesis.semantic_effect_of_operation = Some("A visible state changed".into());
        hypothesis.task_object = None;
        hypothesis.possible_next_action = None;
        for field in [
            "immediate_user_operation",
            "task_object",
            "possible_next_action",
        ] {
            hypothesis.claim_evidence.insert(field.into(), None);
        }
        let mut value = serde_json::to_value(output).unwrap();
        for claim in value["hypotheses"][0]["claim_evidence"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .filter(|claim| !claim.is_null())
        {
            claim["evidence_refs"] = json!(["identity-key"]);
        }
        for contradiction in value["hypotheses"][0]["contradictions"]
            .as_array_mut()
            .unwrap()
        {
            contradiction["evidence_refs"] = json!(["identity-key"]);
        }
        value["hypotheses"][0]["claim_evidence"]["semantic_effect_of_operation"]["evidence_refs"] =
            json!(["identity-key"]);
        for field in ["current_subtask", "likely_primary_task"] {
            value["hypotheses"][0]["claim_evidence"][field]["evidence_refs"] =
                json!(["identity-key"]);
        }
        let identity = EvidenceHandleV2 {
            source_kind: "causal_event".into(),
            record_id: "event-1".into(),
            frame_id: Some("frame-current".into()),
            content_hash: None,
        };
        let parsed = parse_model_response(
            &json!({
                "id":"resp_optional",
                "usage":{"total_tokens":42},
                "output_text":value.to_string()
            }),
            &BTreeMap::from([("identity-key".into(), identity)]),
            Some(&EvidenceKeyPolicyV1 {
                task_identity: BTreeSet::from(["identity-key".into()]),
                ..Default::default()
            }),
        )
        .unwrap();
        assert_eq!(
            parsed.provider_response_id.as_deref(),
            Some("resp_optional")
        );
        assert_eq!(parsed.usage.total_tokens, Some(42));
        assert_eq!(
            parsed.output.hypotheses[0].likely_primary_task.as_deref(),
            Some("Implement cloud task inference")
        );
        assert!(parsed.output.hypotheses[0]
            .semantic_effect_of_operation
            .is_none());
    }

    #[test]
    fn correction_request_names_rejected_field_and_only_its_allowed_keys() {
        let path = image_file();
        let packet = packet(Some(path.to_string_lossy().into_owned()), false);
        let mut request =
            build_multimodal_request(&packet, None, &[], "fixture-model", None).unwrap();
        request
            .evidence_policy
            .task_identity
            .insert("identity-key".into());
        request.evidence_catalog.insert(
            "identity-key".into(),
            EvidenceHandleV2 {
                source_kind: "causal_event".into(),
                record_id: "event-1".into(),
                frame_id: Some("frame-current".into()),
                content_hash: None,
            },
        );
        let failure = ModelFailureV1 {
            kind: ModelFailureKindV1::InvalidJson,
            reason: "evidence_policy_mismatch:current_subtask:task_identity_without_user_authored_or_causal_key".into(),
        };
        let correction = contract_correction(
            &failure,
            &request,
            &json!({"output_text":"{\"schema\":\"rejected\"}"}),
        )
        .unwrap()
        .to_string();
        assert!(correction.contains("current_subtask"));
        assert!(correction.contains("task_identity_without_user_authored_or_causal_key"));
        for allowed in &request.evidence_policy.task_identity {
            assert!(correction.contains(allowed));
        }
        assert!(correction.contains("set the semantic field and claim_evidence entry to null"));
        assert!(correction.contains("previous rejected JSON"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn failed_semantic_validation_preserves_provider_response_identity_and_usage() {
        let path = image_file();
        let client = PreservedFailureClient {
            attempts: vec![ProviderAttemptMetadataV1 {
                response_id: Some("resp_contract_failure".into()),
                request_id: Some("provider_request_contract_failure".into()),
                model: Some("fixture-model".into()),
                usage: ProviderUsageV1 {
                    input_tokens: Some(120),
                    output_tokens: Some(30),
                    total_tokens: Some(150),
                },
            }],
        };
        let attempt = MultimodalTaskTruthResolver.resolve(
            &packet(Some(path.to_string_lossy().into_owned()), false),
            None,
            &[],
            &client,
        );
        assert_eq!(
            attempt.response_id.as_deref(),
            Some("resp_contract_failure")
        );
        assert_eq!(
            attempt.provider_request_id.as_deref(),
            Some("provider_request_contract_failure")
        );
        assert_eq!(attempt.usage.total_tokens, Some(150));
        assert_eq!(attempt.provider_attempts.len(), 1);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn private_current_image_is_blocked_before_client_call() {
        let client = FixtureModelClient {
            model: "fixture-model".into(),
            responses: std::sync::Mutex::new(Vec::new()),
        };
        let attempt = MultimodalTaskTruthResolver.resolve(&packet(None, true), None, &[], &client);
        assert_eq!(attempt.status, ResolutionStatusV1::PrivacyBlocked);
        assert!(attempt.request_audit.is_none());
    }

    #[test]
    fn non_private_missing_current_image_reports_typed_insufficient_evidence() {
        let client = FixtureModelClient {
            model: "fixture-model".into(),
            responses: std::sync::Mutex::new(Vec::new()),
        };
        let mut packet = packet(None, false);
        packet.current_frame.model_eligible = false;
        packet.current_frame.image_rejection_reason =
            Some("full_display_ownership_not_permitted".into());
        packet.semantic_keyframes[0] = packet.current_frame.clone();
        let attempt = MultimodalTaskTruthResolver.resolve(&packet, None, &[], &client);
        assert_eq!(attempt.status, ResolutionStatusV1::InsufficientEvidence);
        assert_eq!(
            attempt.diagnostic_status,
            ProviderDiagnosticStatusV1::RequestInvalid
        );
        assert_eq!(
            attempt.failure.unwrap().reason,
            "full_display_ownership_not_permitted"
        );
    }

    #[test]
    fn provider_timeout_is_typed_and_falls_back_without_fake_output() {
        let path = image_file();
        let client = FixtureModelClient {
            model: "fixture-model".into(),
            responses: std::sync::Mutex::new(vec![Err(ModelFailureV1 {
                kind: ModelFailureKindV1::Timeout,
                reason: "fixture_timeout".into(),
            })]),
        };
        let attempt = MultimodalTaskTruthResolver.resolve(
            &packet(Some(path.to_string_lossy().into_owned()), false),
            None,
            &[],
            &client,
        );
        assert_eq!(attempt.status, ResolutionStatusV1::ProviderFailure);
        assert_eq!(
            attempt.diagnostic_status,
            ProviderDiagnosticStatusV1::Timeout
        );
        assert_eq!(attempt.failure.unwrap().kind, ModelFailureKindV1::Timeout);
        assert!(attempt.output.is_none());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn invalid_json_response_is_a_typed_failure() {
        let result =
            parse_model_response(&json!({"output_text":"not-json"}), &BTreeMap::new(), None);
        assert_eq!(result.unwrap_err().kind, ModelFailureKindV1::InvalidJson);
    }

    #[test]
    fn strict_response_parsing_keeps_provider_identity_and_usage() {
        let output = valid_output(vec![valid_hypothesis("h1")]);
        let parsed = parse_model_response(
            &json!({
                "id": "resp_live_1",
                "request_id": "req_provider_1",
                "model": "fixture-vision-model",
                "usage": {"input_tokens": 1200, "output_tokens": 240, "total_tokens": 1440},
                "output_text": serde_json::to_string(&output).unwrap()
            }),
            &BTreeMap::new(),
            None,
        )
        .unwrap();
        assert_eq!(parsed.provider_response_id.as_deref(), Some("resp_live_1"));
        assert_eq!(
            parsed.provider_request_id.as_deref(),
            Some("req_provider_1")
        );
        assert_eq!(parsed.usage.total_tokens, Some(1440));
        assert_eq!(parsed.output.hypotheses.len(), 1);
    }

    #[test]
    #[ignore = "makes a real provider call with a synthetic repository image"]
    fn live_provider_transport_smoke_uses_only_synthetic_evidence() {
        let config = super::super::super::continue_openai_config(None)
            .expect("load the existing provider configuration");
        let api_key = config
            .api_key
            .expect("configure OPENAI_API_KEY through the existing secure path");
        let image_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("icons")
            .join("32x32.png");
        assert!(image_path.is_file(), "synthetic repository icon is missing");
        let client = OpenAiTaskTruthModelClient::new(config.model, api_key);
        let attempt = MultimodalTaskTruthResolver.resolve(
            &packet(Some(image_path.to_string_lossy().into_owned()), false),
            None,
            &[],
            &client,
        );

        assert_eq!(attempt.origin, InferenceOriginV1::LiveCloud);
        assert_eq!(
            attempt.diagnostic_status,
            ProviderDiagnosticStatusV1::Success,
            "synthetic provider smoke failed safely: {:?}",
            attempt.failure
        );
        assert!(attempt
            .response_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()));
        assert_eq!(
            attempt
                .request_audit
                .as_ref()
                .map(|audit| audit.image_count),
            Some(1)
        );

        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "schema": TASK_TRUTH_MODEL_OUTPUT_SCHEMA_V1,
                "provider": attempt.provider,
                "model": attempt.model,
                "request_id": attempt.request_id,
                "provider_request_id": attempt.provider_request_id,
                "response_id": attempt.response_id,
                "diagnostic_status": attempt.diagnostic_status,
                "resolution_status": attempt.status,
                "latency_ms": attempt.latency_ms,
                "image_count": attempt.request_audit.as_ref().map(|audit| audit.image_count),
                "image_bytes": attempt.request_audit.as_ref().map(|audit| audit.image_bytes),
                "estimated_tokens": attempt.request_audit.as_ref().map(|audit| audit.estimated_tokens),
                "usage": attempt.usage,
                "private_capture_data_sent": false,
            }))
            .expect("serialize safe synthetic smoke metadata")
        );
    }

    #[test]
    fn strict_response_rejects_zero_resolved_or_more_than_three_hypotheses() {
        let empty = TaskTruthModelOutputV1 {
            schema: TASK_TRUTH_MODEL_OUTPUT_SCHEMA_V1.into(),
            resolution_status: ResolutionStatusV1::Resolved,
            hypotheses: Vec::new(),
            missing_evidence: Vec::new(),
            policy_notes: Vec::new(),
        };
        assert!(parse_model_response(
            &json!({
                "output_text": serde_json::to_string(&empty).unwrap()
            }),
            &BTreeMap::new(),
            None,
        )
        .is_err());

        let too_many = valid_output(vec![
            valid_hypothesis("h1"),
            valid_hypothesis("h2"),
            valid_hypothesis("h3"),
            valid_hypothesis("h4"),
        ]);
        assert!(parse_model_response(
            &json!({
                "output_text": serde_json::to_string(&too_many).unwrap()
            }),
            &BTreeMap::new(),
            None,
        )
        .is_err());
    }

    #[test]
    fn strict_response_rejects_unknown_fields_and_invalid_evidence_kinds() {
        let output = valid_output(vec![valid_hypothesis("h1")]);
        let mut value = serde_json::to_value(&output).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), json!(true));
        assert!(parse_model_response(
            &json!({"output_text": value.to_string()}),
            &BTreeMap::new(),
            None,
        )
        .is_err());

        let mut invalid_kind = valid_output(vec![valid_hypothesis("h1")]);
        invalid_kind.hypotheses[0]
            .claim_evidence
            .get_mut("observed_surface")
            .and_then(Option::as_mut)
            .unwrap()
            .evidence_refs[0]
            .source_kind = "browser_chrome_guess".into();
        assert!(parse_model_response(
            &json!({
                "output_text": serde_json::to_string(&invalid_kind).unwrap()
            }),
            &BTreeMap::new(),
            None,
        )
        .is_err());
    }

    #[test]
    fn disabled_and_missing_credentials_have_distinct_diagnostics() {
        let path = image_file();
        for (reason, expected) in [
            ("multimodal_disabled", ProviderDiagnosticStatusV1::Disabled),
            (
                "credentials_missing",
                ProviderDiagnosticStatusV1::CredentialsMissing,
            ),
        ] {
            let client = UnavailableModelClient {
                model: "fixture-model".into(),
                reason: reason.into(),
            };
            let attempt = MultimodalTaskTruthResolver.resolve(
                &packet(Some(path.to_string_lossy().into_owned()), false),
                None,
                &[],
                &client,
            );
            assert_eq!(attempt.status, ResolutionStatusV1::ModelUnavailable);
            assert_eq!(attempt.diagnostic_status, expected);
            assert!(attempt.output.is_none());
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn provider_rejection_timeout_and_model_unavailable_remain_distinct() {
        let rejected = classify_transport_failure("curl failed with HTTP 401".into());
        assert_eq!(rejected.kind, ModelFailureKindV1::RequestRejected);
        assert_eq!(
            diagnostic_for_failure(&rejected),
            ProviderDiagnosticStatusV1::RequestRejected
        );

        let timed_out = classify_transport_failure("request timed out".into());
        assert_eq!(timed_out.kind, ModelFailureKindV1::Timeout);

        let unavailable = classify_transport_failure("HTTP 404 model_not_found".into());
        assert_eq!(unavailable.kind, ModelFailureKindV1::Unavailable);
    }
}
