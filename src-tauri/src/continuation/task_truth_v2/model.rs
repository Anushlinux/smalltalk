use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::time::Instant;

use super::observation_packet::{
    EvidenceHandleV2, EvidencePartitionV2, ObservationPacketV2, RegionRoleV2,
};
use super::task_snapshot::TaskSnapshotV2;

pub(crate) const TASK_TRUTH_MODEL_OUTPUT_SCHEMA_V1: &str = "smalltalk.task_truth_model_output.v1";
pub(crate) const TASK_TRUTH_RESOLVER_VERSION: &str = "task_truth_v2.multimodal_resolver.v1";

const MAX_IMAGES: usize = 4;
const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
const MAX_TOTAL_IMAGE_BYTES: usize = 12 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ResolutionStatusV1 {
    Resolved,
    Ambiguous,
    InsufficientEvidence,
    PrivacyBlocked,
    ModelUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ModelClaimEvidenceV1 {
    pub(crate) field: String,
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
    pub(crate) task_summary: Option<String>,
    pub(crate) task_kind: Option<String>,
    pub(crate) task_object: Option<String>,
    pub(crate) user_goal: Option<String>,
    pub(crate) app_identity: Option<String>,
    pub(crate) surface_identity_hash: Option<String>,
    pub(crate) document_or_thread_identity_hash: Option<String>,
    pub(crate) execution_state: Option<String>,
    pub(crate) current_actor: Option<String>,
    pub(crate) waiting_on: Option<String>,
    pub(crate) last_meaningful_progress: Option<String>,
    pub(crate) unfinished_step: Option<String>,
    pub(crate) next_action: Option<String>,
    pub(crate) relation_to_prior: Option<String>,
    pub(crate) return_anchor_record_id: Option<String>,
    pub(crate) claim_evidence: Vec<ModelClaimEvidenceV1>,
    pub(crate) contradictions: Vec<FieldContradictionV1>,
    pub(crate) confidence_by_field: std::collections::BTreeMap<String, f64>,
    pub(crate) confidence: f64,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct MultimodalRequestAuditV1 {
    pub(crate) request_schema: String,
    pub(crate) model: String,
    pub(crate) image_count: usize,
    pub(crate) image_bytes: usize,
    pub(crate) image_handle_hashes: Vec<String>,
    pub(crate) skipped_images: Vec<String>,
    pub(crate) structured_bytes: usize,
    pub(crate) estimated_tokens: usize,
    pub(crate) max_images: usize,
    pub(crate) max_image_bytes: usize,
    pub(crate) privacy_exclusions: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct TaskTruthModelRequestV1 {
    pub(crate) body: Value,
    pub(crate) audit: MultimodalRequestAuditV1,
}

pub(crate) trait TaskTruthModelClient {
    fn model_name(&self) -> &str;
    fn infer(
        &self,
        request: &TaskTruthModelRequestV1,
    ) -> Result<TaskTruthModelOutputV1, ModelFailureV1>;
}

pub(crate) trait TaskTruthResolver {
    fn resolve(
        &self,
        packet: &ObservationPacketV2,
        prior: Option<&TaskSnapshotV2>,
        client: &dyn TaskTruthModelClient,
    ) -> ResolverAttemptV1;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ResolverAttemptV1 {
    pub(crate) status: ResolutionStatusV1,
    pub(crate) output: Option<TaskTruthModelOutputV1>,
    pub(crate) failure: Option<ModelFailureV1>,
    pub(crate) request_audit: Option<MultimodalRequestAuditV1>,
    pub(crate) latency_ms: i64,
}

pub(crate) struct MultimodalTaskTruthResolver;

impl TaskTruthResolver for MultimodalTaskTruthResolver {
    fn resolve(
        &self,
        packet: &ObservationPacketV2,
        prior: Option<&TaskSnapshotV2>,
        client: &dyn TaskTruthModelClient,
    ) -> ResolverAttemptV1 {
        let started = Instant::now();
        if !packet.current_frame.model_eligible {
            return ResolverAttemptV1 {
                status: ResolutionStatusV1::PrivacyBlocked,
                output: None,
                failure: None,
                request_audit: None,
                latency_ms: started.elapsed().as_millis() as i64,
            };
        }
        let request = match build_multimodal_request(packet, prior, client.model_name(), None) {
            Ok(request) => request,
            Err(failure) => {
                return ResolverAttemptV1 {
                    status: match failure.kind {
                        ModelFailureKindV1::Unavailable => ResolutionStatusV1::ModelUnavailable,
                        _ => ResolutionStatusV1::InsufficientEvidence,
                    },
                    output: None,
                    failure: Some(failure),
                    request_audit: None,
                    latency_ms: started.elapsed().as_millis() as i64,
                }
            }
        };
        let audit = request.audit.clone();
        match client.infer(&request) {
            Ok(output) => ResolverAttemptV1 {
                status: output.resolution_status,
                output: Some(output),
                failure: None,
                request_audit: Some(audit),
                latency_ms: started.elapsed().as_millis() as i64,
            },
            Err(failure) => ResolverAttemptV1 {
                status: if failure.kind == ModelFailureKindV1::Unavailable {
                    ResolutionStatusV1::ModelUnavailable
                } else {
                    ResolutionStatusV1::InsufficientEvidence
                },
                output: None,
                failure: Some(failure),
                request_audit: Some(audit),
                latency_ms: started.elapsed().as_millis() as i64,
            },
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

fn base64_encode(bytes: &[u8]) -> String {
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

pub(crate) fn build_multimodal_request(
    packet: &ObservationPacketV2,
    prior: Option<&TaskSnapshotV2>,
    model: &str,
    reconciliation: Option<&Value>,
) -> Result<TaskTruthModelRequestV1, ModelFailureV1> {
    let structured = json!({
        "packet": packet,
        "previous_valid_snapshot": prior,
        "reconciliation": reconciliation,
        "request_policy": {
            "explicit_manual_continue": true,
            "background_upload_allowed": false,
            "target_selection_authority": false,
            "images_are_active_window_crops": true,
        }
    });
    let structured_text = serde_json::to_string(&structured).map_err(|_| ModelFailureV1 {
        kind: ModelFailureKindV1::InvalidJson,
        reason: "structured_packet_serialization_failed".into(),
    })?;
    let mut content = vec![json!({"type":"input_text", "text": structured_text})];
    let mut image_bytes = 0usize;
    let mut image_count = 0usize;
    let mut image_hashes = Vec::new();
    let mut skipped = Vec::new();
    let mut privacy_exclusions = Vec::new();
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
            privacy_exclusions.push(format!("{}:privacy_blocked", frame.frame_id));
            continue;
        }
        let Some(raw_path) = frame.ephemeral_local_image_path.as_deref() else {
            skipped.push(format!("{}:missing_ephemeral_image", frame.frame_id));
            continue;
        };
        let path = Path::new(raw_path);
        let Some(mime) = mime_type(path) else {
            skipped.push(format!("{}:unsupported_image_type", frame.frame_id));
            continue;
        };
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(_) => {
                skipped.push(format!("{}:unreadable_image", frame.frame_id));
                continue;
            }
        };
        if bytes.len() > MAX_IMAGE_BYTES || image_bytes + bytes.len() > MAX_TOTAL_IMAGE_BYTES {
            skipped.push(format!("{}:image_byte_cap", frame.frame_id));
            continue;
        }
        content.push(json!({
            "type": "input_image",
            "image_url": format!("data:{mime};base64,{}", base64_encode(&bytes)),
            "detail": "high"
        }));
        image_count += 1;
        image_bytes += bytes.len();
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
    let body = json!({
        "model": model,
        "store": false,
        "max_output_tokens": 1800,
        "text": {"format": {
            "type": "json_schema",
            "name": "smalltalk_task_truth_resolution",
            "strict": true,
            "schema": model_output_schema()
        }},
        "input": [
            {"role":"system", "content":[{"type":"input_text", "text": system_instruction()}]},
            {"role":"user", "content": content}
        ]
    });
    Ok(TaskTruthModelRequestV1 {
        body,
        audit: MultimodalRequestAuditV1 {
            request_schema: "smalltalk.task_truth_multimodal_request.v1".into(),
            model: model.into(),
            image_count,
            image_bytes,
            image_handle_hashes: image_hashes,
            skipped_images: skipped,
            structured_bytes: structured_text.len(),
            estimated_tokens: structured_text.len().div_ceil(4) + image_count * 1100,
            max_images: MAX_IMAGES,
            max_image_bytes: MAX_IMAGE_BYTES,
            privacy_exclusions,
        },
    })
}

fn system_instruction() -> &'static str {
    "You are a semantic sensor for the current user task. Inspect the supplied real images and structured temporal evidence. Identify the primary task-bearing region; separate user-created content, application or agent output, controls, navigation, and third-party content. Interaction causality is stronger authorship evidence than geometry. Infer the goal and work object; distinguish composing, editing, reviewing, waiting, debugging, searching, comparing, blocked, and completed states. Identify last meaningful progress and unfinished work. Give a next action only when it follows from that unfinished state. Keep two bounded hypotheses when interpretations are close. Keep return anchors separate from task understanding. Every material field must cite only element, frame, event, transition, or previous-snapshot ids present in the request. Controls, navigation, and browser chrome cannot be authored goals. Do not invent identifiers, objects, or actions. Return strict JSON only."
}

fn nullable_string() -> Value {
    json!({"type":["string","null"]})
}

fn evidence_ref_schema() -> Value {
    json!({
        "type":"object", "additionalProperties":false,
        "required":["source_kind","record_id","frame_id","content_hash"],
        "properties":{
            "source_kind":{"type":"string"}, "record_id":{"type":"string"},
            "frame_id":nullable_string(), "content_hash":nullable_string()
        }
    })
}

fn hypothesis_schema() -> Value {
    let nullable = nullable_string();
    json!({
        "type":"object", "additionalProperties":false,
        "required":["hypothesis_id","task_summary","task_kind","task_object","user_goal","app_identity","surface_identity_hash","document_or_thread_identity_hash","execution_state","current_actor","waiting_on","last_meaningful_progress","unfinished_step","next_action","relation_to_prior","return_anchor_record_id","claim_evidence","contradictions","confidence_by_field","confidence"],
        "properties":{
            "hypothesis_id":{"type":"string"}, "task_summary":nullable, "task_kind":nullable_string(),
            "task_object":nullable_string(), "user_goal":nullable_string(), "app_identity":nullable_string(),
            "surface_identity_hash":nullable_string(), "document_or_thread_identity_hash":nullable_string(),
            "execution_state":nullable_string(), "current_actor":nullable_string(), "waiting_on":nullable_string(),
            "last_meaningful_progress":nullable_string(), "unfinished_step":nullable_string(), "next_action":nullable_string(),
            "relation_to_prior":nullable_string(), "return_anchor_record_id":nullable_string(),
            "claim_evidence":{"type":"array","maxItems":20,"items":{
                "type":"object","additionalProperties":false,"required":["field","claim","evidence_refs","confidence"],
                "properties":{"field":{"type":"string"},"claim":{"type":"string"},"evidence_refs":{"type":"array","maxItems":8,"items":evidence_ref_schema()},"confidence":{"type":"number","minimum":0,"maximum":1}}
            }},
            "contradictions":{"type":"array","maxItems":12,"items":{
                "type":"object","additionalProperties":false,"required":["field","reason","evidence_refs"],
                "properties":{"field":{"type":"string"},"reason":{"type":"string"},"evidence_refs":{"type":"array","maxItems":8,"items":evidence_ref_schema()}}
            }},
            "confidence_by_field":{
                "type":"object","additionalProperties":false,
                "required":["task_summary","task_kind","task_object","user_goal","app_identity","surface_identity_hash","document_or_thread_identity_hash","execution_state","current_actor","waiting_on","last_meaningful_progress","unfinished_step","next_action","relation_to_prior"],
                "properties":{
                    "task_summary":{"type":"number","minimum":0,"maximum":1},
                    "task_kind":{"type":"number","minimum":0,"maximum":1},
                    "task_object":{"type":"number","minimum":0,"maximum":1},
                    "user_goal":{"type":"number","minimum":0,"maximum":1},
                    "app_identity":{"type":"number","minimum":0,"maximum":1},
                    "surface_identity_hash":{"type":"number","minimum":0,"maximum":1},
                    "document_or_thread_identity_hash":{"type":"number","minimum":0,"maximum":1},
                    "execution_state":{"type":"number","minimum":0,"maximum":1},
                    "current_actor":{"type":"number","minimum":0,"maximum":1},
                    "waiting_on":{"type":"number","minimum":0,"maximum":1},
                    "last_meaningful_progress":{"type":"number","minimum":0,"maximum":1},
                    "unfinished_step":{"type":"number","minimum":0,"maximum":1},
                    "next_action":{"type":"number","minimum":0,"maximum":1},
                    "relation_to_prior":{"type":"number","minimum":0,"maximum":1}
                }
            },
            "confidence":{"type":"number","minimum":0,"maximum":1}
        }
    })
}

fn model_output_schema() -> Value {
    json!({
        "type":"object","additionalProperties":false,
        "required":["schema","resolution_status","hypotheses","missing_evidence","policy_notes"],
        "properties":{
            "schema":{"type":"string","enum":[TASK_TRUTH_MODEL_OUTPUT_SCHEMA_V1]},
            "resolution_status":{"type":"string","enum":["resolved","ambiguous","insufficient_evidence","privacy_blocked","model_unavailable"]},
            "hypotheses":{"type":"array","maxItems":2,"items":hypothesis_schema()},
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
    fn model_name(&self) -> &str {
        &self.model
    }

    fn infer(&self, _: &TaskTruthModelRequestV1) -> Result<TaskTruthModelOutputV1, ModelFailureV1> {
        Err(ModelFailureV1 {
            kind: ModelFailureKindV1::Unavailable,
            reason: self.reason.clone(),
        })
    }
}

pub(crate) struct OpenAiTaskTruthModelClient {
    pub(crate) model: String,
    pub(crate) api_key: String,
}

impl TaskTruthModelClient for OpenAiTaskTruthModelClient {
    fn model_name(&self) -> &str {
        &self.model
    }

    fn infer(
        &self,
        request: &TaskTruthModelRequestV1,
    ) -> Result<TaskTruthModelOutputV1, ModelFailureV1> {
        let response =
            super::super::call_openai_responses_with_timeout(&self.api_key, &request.body, 90, 1)
                .map_err(classify_transport_failure)?;
        parse_model_response(&response)
    }
}

fn classify_transport_failure(error: String) -> ModelFailureV1 {
    let lower = error.to_ascii_lowercase();
    let kind = if lower.contains("timeout") || lower.contains("timed out") {
        ModelFailureKindV1::Timeout
    } else {
        ModelFailureKindV1::ProviderError
    };
    ModelFailureV1 {
        kind,
        reason: "provider_request_failed".into(),
    }
}

fn parse_model_response(response: &Value) -> Result<TaskTruthModelOutputV1, ModelFailureV1> {
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
    let output: TaskTruthModelOutputV1 =
        serde_json::from_str(&text).map_err(|_| ModelFailureV1 {
            kind: ModelFailureKindV1::InvalidJson,
            reason: "invalid_structured_output".into(),
        })?;
    if output.schema != TASK_TRUTH_MODEL_OUTPUT_SCHEMA_V1 || output.hypotheses.len() > 2 {
        return Err(ModelFailureV1 {
            kind: ModelFailureKindV1::InvalidJson,
            reason: "schema_contract_mismatch".into(),
        });
    }
    Ok(output)
}

#[cfg(test)]
pub(crate) struct FixtureModelClient {
    pub(crate) model: String,
    pub(crate) responses: std::sync::Mutex<Vec<Result<TaskTruthModelOutputV1, ModelFailureV1>>>,
}

#[cfg(test)]
impl TaskTruthModelClient for FixtureModelClient {
    fn model_name(&self) -> &str {
        &self.model
    }

    fn infer(&self, _: &TaskTruthModelRequestV1) -> Result<TaskTruthModelOutputV1, ModelFailureV1> {
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

    fn packet(path: Option<String>, private: bool) -> ObservationPacketV2 {
        let frame = KeyframeReferenceV2 {
            frame_id: "frame-current".into(),
            observed_at_ms: 1_000,
            partition: EvidencePartitionV2::Current,
            privacy_status: if private { "private" } else { "allowed" }.into(),
            model_eligible: !private,
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
            },
        }
    }

    fn image_file() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "smalltalk-task-truth-model-{}.png",
            std::process::id()
        ));
        // The transport only needs real image bytes here; provider decoding is not used in tests.
        fs::write(&path, b"\x89PNG\r\n\x1a\nfixture").unwrap();
        path
    }

    #[test]
    fn request_contains_real_image_input_but_serialized_packet_omits_local_path() {
        let path = image_file();
        let packet = packet(Some(path.to_string_lossy().into_owned()), false);
        let request = build_multimodal_request(&packet, None, "fixture-model", None).unwrap();
        assert_eq!(request.audit.image_count, 1);
        assert_eq!(
            request.audit.image_handle_hashes,
            vec!["durable-image-hash"]
        );
        let body = request.body.to_string();
        assert!(body.contains("data:image/png;base64,"));
        assert!(!body.contains(path.to_string_lossy().as_ref()));
        assert!(!serde_json::to_string(&packet)
            .unwrap()
            .contains(path.to_string_lossy().as_ref()));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn private_current_image_is_blocked_before_client_call() {
        let client = FixtureModelClient {
            model: "fixture-model".into(),
            responses: std::sync::Mutex::new(Vec::new()),
        };
        let attempt = MultimodalTaskTruthResolver.resolve(&packet(None, true), None, &client);
        assert_eq!(attempt.status, ResolutionStatusV1::PrivacyBlocked);
        assert!(attempt.request_audit.is_none());
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
            &client,
        );
        assert_eq!(attempt.status, ResolutionStatusV1::InsufficientEvidence);
        assert_eq!(attempt.failure.unwrap().kind, ModelFailureKindV1::Timeout);
        assert!(attempt.output.is_none());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn invalid_json_response_is_a_typed_failure() {
        let result = parse_model_response(&json!({"output_text":"not-json"}));
        assert_eq!(result.unwrap_err().kind, ModelFailureKindV1::InvalidJson);
    }
}
