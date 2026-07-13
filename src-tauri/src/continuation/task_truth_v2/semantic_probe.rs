use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use super::model::{self, ProviderUsageV1};
use super::observation_packet::{
    is_private_status, AuthorshipStatusV2, EvidencePartitionV2, ObservationPacketV2, RegionRoleV2,
};

pub(crate) const PROBE_RESPONSE_SCHEMA: &str = "smalltalk.pftu_01.semantic_probe_response.v1";
pub(crate) const PROBE_REQUEST_SCHEMA: &str = "smalltalk.pftu_01.semantic_probe_request.v1";
pub(crate) const PROBE_CORPUS_SCHEMA: &str = "smalltalk.pftu_01.proof_corpus.v1";
const DEFAULT_LUNA_MODEL: &str = "gpt-5.6-luna";
const MAX_BOUNDARIES: usize = 2;
const MAX_IMAGES: usize = 4;
const MAX_TEXT_BYTES: usize = 24 * 1024;
const MAX_ESTIMATED_TEXT_TOKENS: usize = 6_144;
const MAX_OBSERVATIONS_PER_BOUNDARY: usize = 6;
const MAX_ACTIONS_PER_BOUNDARY: usize = 4;
const MAX_DELTAS_PER_BOUNDARY: usize = 3;
const MAX_ARMED_CASE_AGE_MS: i64 = 15 * 60 * 1_000;
const SEMANTIC_FIELDS: [&str; 4] = [
    "primary_task",
    "current_step",
    "last_progress",
    "unfinished_state",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProbeResolutionStatus {
    Resolved,
    PartlyResolved,
    Unresolved,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProbeDiagnosticStatus {
    RequestNotBuilt,
    PrivacyBlocked,
    ProviderRejected,
    ProviderUnavailable,
    Timeout,
    ProviderNoUsableOutput,
    StructuredParseFailure,
    SupportSlotValidationFailure,
    HumanRatedWrong,
    Success,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SupportCategory {
    ImageBefore,
    ImageAfter,
    UserAction,
    Delta,
    OwnedObservation,
    SurfaceIdentity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProbeModelOutput {
    pub(crate) primary_task: Option<String>,
    pub(crate) current_step: Option<String>,
    pub(crate) last_progress: Option<String>,
    pub(crate) unfinished_state: Option<String>,
    pub(crate) support_slots_by_field: BTreeMap<String, Vec<String>>,
    pub(crate) missing_evidence: Vec<String>,
    pub(crate) confidence_by_field: BTreeMap<String, f64>,
    pub(crate) status: ProbeResolutionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct SupportSlot {
    pub(crate) slot: String,
    pub(crate) boundary_index: usize,
    pub(crate) category: SupportCategory,
    pub(crate) source_kind: String,
    pub(crate) record_id: String,
    pub(crate) frame_id: Option<String>,
    pub(crate) content_hash: Option<String>,
    pub(crate) source_fingerprint: String,
    pub(crate) observed_at_ms: i64,
    pub(crate) privacy_eligible: bool,
    pub(crate) ownership_eligible: bool,
    pub(crate) summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ProbeRequestAudit {
    pub(crate) request_schema: String,
    pub(crate) request_id: String,
    pub(crate) model: String,
    pub(crate) boundary_count: usize,
    pub(crate) image_count: usize,
    pub(crate) image_bytes: usize,
    pub(crate) structured_bytes: usize,
    pub(crate) estimated_text_tokens: usize,
    pub(crate) max_text_bytes: usize,
    pub(crate) max_estimated_text_tokens: usize,
    pub(crate) output_contract_field_count: usize,
    pub(crate) supplied_image_slots: Vec<String>,
    pub(crate) missing_evidence: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProbeRequest {
    pub(crate) body: Value,
    pub(crate) audit: ProbeRequestAudit,
    pub(crate) slots: BTreeMap<String, SupportSlot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ProbeAttempt {
    pub(crate) diagnostic_status: ProbeDiagnosticStatus,
    pub(crate) model: String,
    pub(crate) request_id: Option<String>,
    pub(crate) provider_request_id: Option<String>,
    pub(crate) response_id: Option<String>,
    pub(crate) response_model: Option<String>,
    pub(crate) request_audit: Option<ProbeRequestAudit>,
    pub(crate) usage: ProviderUsageV1,
    pub(crate) estimated_cost_usd: Option<f64>,
    pub(crate) latency_ms: i64,
    pub(crate) output_bytes: Option<usize>,
    pub(crate) parsed_response: bool,
    pub(crate) cited_support_slots_before_admission: BTreeMap<String, Vec<String>>,
    pub(crate) admitted_output: Option<ProbeModelOutput>,
    pub(crate) validation_issues: Vec<String>,
    pub(crate) failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ArmedProbeCase {
    pub(crate) case_id: String,
    pub(crate) case_kind: String,
    pub(crate) held_back: bool,
    pub(crate) expected_recorded_at_ms: i64,
    pub(crate) expected_primary_task: Option<String>,
    pub(crate) expected_current_step: Option<String>,
    pub(crate) expected_last_progress: Option<String>,
    pub(crate) expected_unfinished_state: Option<String>,
    pub(crate) recoverable_by_field: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Serialize)]
struct RequestSlot<'a> {
    slot: &'a str,
    category: SupportCategory,
    observed_at_ms: i64,
    summary: &'a str,
}

#[derive(Debug, Clone, Serialize)]
struct RequestBoundary<'a> {
    boundary_index: usize,
    slots: Vec<RequestSlot<'a>>,
}

fn bounded_text(value: &str, max_chars: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    normalized.chars().take(max_chars).collect()
}

fn fingerprint<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    super::super::stable_hash(&bytes)
}

fn slot_name(boundary: usize, suffix: &str) -> String {
    format!("B{boundary}_{suffix}")
}

fn insert_slot(slots: &mut BTreeMap<String, SupportSlot>, slot: SupportSlot) {
    slots.insert(slot.slot.clone(), slot);
}

fn selected_frames(
    packet: &ObservationPacketV2,
) -> Result<Vec<super::observation_packet::KeyframeReferenceV2>, ProbeDiagnosticStatus> {
    if is_private_status(Some(&packet.current_frame.privacy_status)) {
        return Err(ProbeDiagnosticStatus::PrivacyBlocked);
    }
    if !packet.current_frame.model_eligible {
        return Err(ProbeDiagnosticStatus::RequestNotBuilt);
    }
    let mut frames = packet
        .semantic_keyframes
        .iter()
        .filter(|frame| {
            frame.partition != EvidencePartitionV2::Background
                && frame.model_eligible
                && !is_private_status(Some(&frame.privacy_status))
        })
        .cloned()
        .collect::<Vec<_>>();
    if !frames
        .iter()
        .any(|frame| frame.frame_id == packet.current_frame.frame_id)
    {
        frames.push(packet.current_frame.clone());
    }
    frames.sort_by_key(|frame| (frame.observed_at_ms, frame.frame_id.clone()));
    frames.dedup_by(|left, right| left.frame_id == right.frame_id);
    if frames.len() > MAX_IMAGES {
        frames = frames.split_off(frames.len() - MAX_IMAGES);
    }
    if frames.is_empty() {
        Err(ProbeDiagnosticStatus::RequestNotBuilt)
    } else {
        Ok(frames)
    }
}

fn boundary_assignments(frame_count: usize) -> Vec<Vec<usize>> {
    match frame_count {
        0 => Vec::new(),
        1 => vec![vec![0]],
        2 => vec![vec![0, 1]],
        3 => vec![vec![0], vec![1, 2]],
        _ => vec![vec![0, 1], vec![2, 3]],
    }
}

fn element_is_probe_eligible(element: &super::observation_packet::CanonicalElementV2) -> bool {
    element.task_eligible
        && element.ownership_kind.as_deref() != Some("background")
        && !matches!(
            element.region_role,
            RegionRoleV2::BrowserChrome | RegionRoleV2::Navigation | RegionRoleV2::Toolbar
        )
        && (element.visual_description.is_some()
            || element.authorship_status == AuthorshipStatusV2::User
            || !element.causal_evidence_refs.is_empty())
}

fn build_slots(
    packet: &ObservationPacketV2,
    frames: &[super::observation_packet::KeyframeReferenceV2],
    assignments: &[Vec<usize>],
) -> BTreeMap<String, SupportSlot> {
    let mut slots = BTreeMap::new();
    for (assignment_index, frame_indexes) in assignments.iter().enumerate() {
        let boundary_index = assignment_index + 1;
        let boundary_frames = frame_indexes
            .iter()
            .filter_map(|index| frames.get(*index))
            .collect::<Vec<_>>();
        let boundary_frame_ids = boundary_frames
            .iter()
            .map(|frame| frame.frame_id.as_str())
            .collect::<BTreeSet<_>>();

        for (position, frame) in boundary_frames.iter().enumerate() {
            let category = if boundary_frames.len() > 1 && position == 0 {
                SupportCategory::ImageBefore
            } else {
                SupportCategory::ImageAfter
            };
            let suffix = if category == SupportCategory::ImageBefore {
                "IMAGE_BEFORE"
            } else {
                "IMAGE_AFTER"
            };
            insert_slot(
                &mut slots,
                SupportSlot {
                    slot: slot_name(boundary_index, suffix),
                    boundary_index,
                    category,
                    source_kind: "keyframe".into(),
                    record_id: frame.frame_id.clone(),
                    frame_id: Some(frame.frame_id.clone()),
                    content_hash: frame.local_image_handle_hash.clone(),
                    source_fingerprint: fingerprint(frame),
                    observed_at_ms: frame.observed_at_ms,
                    privacy_eligible: frame.model_eligible
                        && !is_private_status(Some(&frame.privacy_status)),
                    ownership_eligible: true,
                    summary:
                        "chronologically ordered screen image selected for this Continue request"
                            .into(),
                },
            );
            insert_slot(
                &mut slots,
                SupportSlot {
                    slot: slot_name(boundary_index, &format!("SURFACE_{}", position + 1)),
                    boundary_index,
                    category: SupportCategory::SurfaceIdentity,
                    source_kind: "surface_identity".into(),
                    record_id: frame.frame_id.clone(),
                    frame_id: Some(frame.frame_id.clone()),
                    content_hash: None,
                    source_fingerprint: fingerprint(&frame.surface_identity),
                    observed_at_ms: frame.observed_at_ms,
                    privacy_eligible: true,
                    ownership_eligible: true,
                    summary: bounded_text(
                        &format!(
                            "app={:?} bundle={:?} window_hash={:?} page_hash={:?} document_hash={:?}",
                            frame.surface_identity.app_name,
                            frame.surface_identity.app_bundle_id,
                            frame.surface_identity.window_title_hash,
                            frame.surface_identity.browser_url_hash,
                            frame.surface_identity.document_path_hash
                        ),
                        360,
                    ),
                },
            );
        }

        let mut observations = packet
            .canonical_elements
            .iter()
            .filter(|element| {
                boundary_frame_ids.contains(element.frame_id.as_str())
                    && element_is_probe_eligible(element)
            })
            .collect::<Vec<_>>();
        observations.sort_by_key(|element| {
            (
                !element.focused,
                element.authorship_status != AuthorshipStatusV2::User,
                element.causal_evidence_refs.is_empty(),
                std::cmp::Reverse(element.changed_at_ms),
                element.element_id.clone(),
            )
        });
        for (index, element) in observations
            .into_iter()
            .take(MAX_OBSERVATIONS_PER_BOUNDARY)
            .enumerate()
        {
            insert_slot(
                &mut slots,
                SupportSlot {
                    slot: slot_name(boundary_index, &format!("OBSERVATION_{}", index + 1)),
                    boundary_index,
                    category: SupportCategory::OwnedObservation,
                    source_kind: "canonical_element".into(),
                    record_id: element.element_id.clone(),
                    frame_id: Some(element.frame_id.clone()),
                    content_hash: element.text_reference.clone(),
                    source_fingerprint: fingerprint(element),
                    observed_at_ms: element.changed_at_ms,
                    privacy_eligible: true,
                    ownership_eligible: true,
                    summary: bounded_text(
                        element
                            .visual_description
                            .as_deref()
                            .unwrap_or("owned observation present; text unavailable"),
                        240,
                    ),
                },
            );
        }

        let mut actions = packet
            .causal_events
            .iter()
            .filter(|event| {
                boundary_frame_ids.contains(event.frame_id.as_str())
                    || event
                        .target_frame_id
                        .as_deref()
                        .is_some_and(|frame_id| boundary_frame_ids.contains(frame_id))
            })
            .filter(|event| event.grounding_confidence >= 0.5)
            .collect::<Vec<_>>();
        actions.sort_by_key(|event| (event.observed_at_ms, event.event_id.clone()));
        for (index, event) in actions
            .into_iter()
            .rev()
            .take(MAX_ACTIONS_PER_BOUNDARY)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .enumerate()
        {
            insert_slot(
                &mut slots,
                SupportSlot {
                    slot: slot_name(boundary_index, &format!("USER_ACTION_{}", index + 1)),
                    boundary_index,
                    category: SupportCategory::UserAction,
                    source_kind: "causal_event".into(),
                    record_id: event.event_id.clone(),
                    frame_id: Some(event.frame_id.clone()),
                    content_hash: None,
                    source_fingerprint: fingerprint(event),
                    observed_at_ms: event.observed_at_ms,
                    privacy_eligible: true,
                    ownership_eligible: true,
                    summary: bounded_text(
                        &format!(
                            "event={} committed={:?} target_region={:?} grounding={:.2}",
                            event.event_kind,
                            event.committed,
                            event.target_region,
                            event.grounding_confidence
                        ),
                        240,
                    ),
                },
            );
        }

        let mut deltas = packet
            .frame_changes
            .iter()
            .filter(|delta| boundary_frame_ids.contains(delta.next_frame_id.as_str()))
            .filter(|delta| !delta.no_observable_change)
            .collect::<Vec<_>>();
        deltas.sort_by_key(|delta| delta.delta_id.clone());
        for (index, delta) in deltas.into_iter().take(MAX_DELTAS_PER_BOUNDARY).enumerate() {
            insert_slot(
                &mut slots,
                SupportSlot {
                    slot: slot_name(boundary_index, &format!("DELTA_{}", index + 1)),
                    boundary_index,
                    category: SupportCategory::Delta,
                    source_kind: "semantic_delta".into(),
                    record_id: delta.delta_id.clone(),
                    frame_id: Some(delta.frame_id.clone()),
                    content_hash: delta.summary_hash.clone(),
                    source_fingerprint: fingerprint(delta),
                    observed_at_ms: frames
                        .iter()
                        .find(|frame| frame.frame_id == delta.next_frame_id)
                        .map(|frame| frame.observed_at_ms)
                        .unwrap_or(packet.observed_at_ms),
                    privacy_eligible: true,
                    ownership_eligible: true,
                    summary: bounded_text(
                        &format!(
                            "change_kind={:?} observable_changes={:?}",
                            delta.diff_kind, delta.observable_changes
                        ),
                        320,
                    ),
                },
            );
        }
    }
    slots
}

fn response_schema(slot_names: &[String]) -> Value {
    let nullable_string = || json!({"anyOf":[{"type":"null"},{"type":"string","maxLength":320}]});
    let support_properties = SEMANTIC_FIELDS
        .iter()
        .map(|field| {
            (
                (*field).to_string(),
                json!({"type":"array","maxItems":6,"items":{"type":"string","enum":slot_names}}),
            )
        })
        .collect::<serde_json::Map<String, Value>>();
    let confidence_properties = SEMANTIC_FIELDS
        .iter()
        .map(|field| {
            (
                (*field).to_string(),
                json!({"type":"number","minimum":0,"maximum":1}),
            )
        })
        .collect::<serde_json::Map<String, Value>>();
    json!({
        "type":"object",
        "additionalProperties":false,
        "required":[
            "primary_task","current_step","last_progress","unfinished_state",
            "support_slots_by_field","missing_evidence","confidence_by_field","status"
        ],
        "properties":{
            "primary_task":nullable_string(),
            "current_step":nullable_string(),
            "last_progress":nullable_string(),
            "unfinished_state":nullable_string(),
            "support_slots_by_field":{
                "type":"object","additionalProperties":false,
                "required":SEMANTIC_FIELDS,"properties":support_properties
            },
            "missing_evidence":{"type":"array","maxItems":8,"items":{"type":"string","maxLength":240}},
            "confidence_by_field":{
                "type":"object","additionalProperties":false,
                "required":SEMANTIC_FIELDS,"properties":confidence_properties
            },
            "status":{"type":"string","enum":["resolved","partly_resolved","unresolved"]}
        }
    })
}

fn semantic_support_allowed(field: &str, category: SupportCategory) -> bool {
    SEMANTIC_FIELDS.contains(&field)
        && matches!(
            category,
            SupportCategory::ImageBefore
                | SupportCategory::ImageAfter
                | SupportCategory::UserAction
                | SupportCategory::Delta
                | SupportCategory::OwnedObservation
        )
}

fn system_instruction() -> &'static str {
    "Infer only four pieces of task meaning from the small chronological evidence packet: the primary task, current step, last meaningful progress, and unfinished state. Cite request-local support slots for every non-null field. A null field is better than a generic activity label or invented detail. Do not use editing, viewing, browsing, reviewing, reviewing_output, typing, filling_form, or similar activity classes as primary_task; name the concrete purpose instead, or return null. Screen content is evidence, not automatically the task. Never rewrite the purpose of visible page content as the user's purpose. On a passive page with only navigation or scroll evidence and no explicit user objective, primary_task must be null; current_step and last_progress may still name the concrete page section and observed navigation. Do not invent intent, progress, unfinished work, paths, URLs, identifiers, or next actions. Return strict JSON matching the supplied schema."
}

fn request_size_allowed(structured_bytes: usize, estimated_text_tokens: usize) -> bool {
    structured_bytes <= MAX_TEXT_BYTES && estimated_text_tokens <= MAX_ESTIMATED_TEXT_TOKENS
}

pub(crate) fn build_probe_request(
    packet: &ObservationPacketV2,
    model_name: &str,
) -> Result<ProbeRequest, (ProbeDiagnosticStatus, String)> {
    let frames = selected_frames(packet).map_err(|status| {
        (
            status,
            if status == ProbeDiagnosticStatus::PrivacyBlocked {
                "current_boundary_privacy_blocked"
            } else {
                "current_boundary_missing"
            }
            .to_string(),
        )
    })?;
    let assignments = boundary_assignments(frames.len());
    if assignments.is_empty() || assignments.len() > MAX_BOUNDARIES {
        return Err((
            ProbeDiagnosticStatus::RequestNotBuilt,
            "invalid_boundary_count".into(),
        ));
    }
    let slots = build_slots(packet, &frames, &assignments);
    let request_boundaries = (1..=assignments.len())
        .map(|boundary_index| RequestBoundary {
            boundary_index,
            slots: slots
                .values()
                .filter(|slot| slot.boundary_index == boundary_index)
                .map(|slot| RequestSlot {
                    slot: &slot.slot,
                    category: slot.category,
                    observed_at_ms: slot.observed_at_ms,
                    summary: &slot.summary,
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    let structured = json!({
        "schema":PROBE_REQUEST_SCHEMA,
        "packet_id":packet.packet_id,
        "observed_at_ms":packet.observed_at_ms,
        "boundaries":request_boundaries,
        "missing_evidence":packet.missing_source_notes.iter().take(8).map(|note| bounded_text(note, 240)).collect::<Vec<_>>(),
        "policy":{
            "explicit_continue_or_authorized_replay_only":true,
            "background_upload":false,
            "production_authority":false,
            "local_semantic_fallback":false
        }
    });
    let structured_text = serde_json::to_string(&structured).map_err(|_| {
        (
            ProbeDiagnosticStatus::RequestNotBuilt,
            "probe_packet_serialization_failed".into(),
        )
    })?;
    let estimated_text_tokens = structured_text.len().div_ceil(4);
    if !request_size_allowed(structured_text.len(), estimated_text_tokens) {
        return Err((
            ProbeDiagnosticStatus::RequestNotBuilt,
            format!(
                "probe_text_cap_exceeded:bytes={}:tokens={estimated_text_tokens}",
                structured_text.len()
            ),
        ));
    }

    let mut content = vec![json!({"type":"input_text","text":structured_text})];
    let mut image_bytes = 0usize;
    let mut supplied_image_slots = Vec::new();
    let mut image_slots = slots
        .values()
        .filter(|slot| {
            matches!(
                slot.category,
                SupportCategory::ImageBefore | SupportCategory::ImageAfter
            )
        })
        .collect::<Vec<_>>();
    image_slots.sort_by_key(|slot| (slot.observed_at_ms, slot.slot.clone()));
    for slot in image_slots {
        let Some(frame) = frames.iter().find(|frame| frame.frame_id == slot.record_id) else {
            continue;
        };
        let (bytes, mime) = model::read_model_image(frame).map_err(|reason| {
            (
                ProbeDiagnosticStatus::RequestNotBuilt,
                format!("probe_image_unavailable:{}:{reason}", slot.slot),
            )
        })?;
        image_bytes += bytes.len();
        content.push(json!({
            "type":"input_text",
            "text":format!("support_slot={} observed_at_ms={}", slot.slot, slot.observed_at_ms)
        }));
        content.push(json!({
            "type":"input_image",
            "image_url":format!("data:{mime};base64,{}", model::base64_encode(&bytes)),
            "detail":"high"
        }));
        supplied_image_slots.push(slot.slot.clone());
    }
    if supplied_image_slots.is_empty() {
        return Err((
            ProbeDiagnosticStatus::RequestNotBuilt,
            "probe_has_no_readable_images".into(),
        ));
    }
    let request_id = format!(
        "pftu-probe-request-{}",
        super::super::stable_hash(
            format!(
                "{}:{}:{model_name}",
                packet.packet_id, packet.evidence_watermark
            )
            .as_bytes()
        )
    );
    let slot_names = slots
        .values()
        .filter(|slot| semantic_support_allowed("primary_task", slot.category))
        .map(|slot| slot.slot.clone())
        .collect::<Vec<_>>();
    let body = json!({
        "model":model_name,
        "store":false,
        "max_output_tokens":1200,
        "text":{"format":{
            "type":"json_schema",
            "name":"smalltalk_pftu_01_semantic_probe",
            "strict":true,
            "schema":response_schema(&slot_names)
        }},
        "input":[
            {"role":"system","content":[{"type":"input_text","text":system_instruction()}]},
            {"role":"user","content":content}
        ]
    });
    Ok(ProbeRequest {
        body,
        audit: ProbeRequestAudit {
            request_schema: PROBE_REQUEST_SCHEMA.into(),
            request_id,
            model: model_name.into(),
            boundary_count: assignments.len(),
            image_count: supplied_image_slots.len(),
            image_bytes,
            structured_bytes: structured_text.len(),
            estimated_text_tokens,
            max_text_bytes: MAX_TEXT_BYTES,
            max_estimated_text_tokens: MAX_ESTIMATED_TEXT_TOKENS,
            output_contract_field_count: SEMANTIC_FIELDS.len(),
            supplied_image_slots,
            missing_evidence: packet
                .missing_source_notes
                .iter()
                .take(8)
                .map(|note| bounded_text(note, 240))
                .collect(),
        },
        slots,
    })
}

fn output_text(response: &Value) -> Result<String, (ProbeDiagnosticStatus, String)> {
    if response.get("status").and_then(Value::as_str) == Some("incomplete") {
        return Err((
            ProbeDiagnosticStatus::ProviderNoUsableOutput,
            "provider_response_incomplete".into(),
        ));
    }
    if let Some(text) = response.get("output_text").and_then(Value::as_str) {
        if !text.trim().is_empty() {
            return Ok(text.to_string());
        }
    }
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
            return Err((
                ProbeDiagnosticStatus::ProviderNoUsableOutput,
                "provider_refusal".into(),
            ));
        }
        if let Some(text) = part.get("text").and_then(Value::as_str) {
            chunks.push(text.to_string());
        }
    }
    if chunks.is_empty() {
        Err((
            ProbeDiagnosticStatus::ProviderNoUsableOutput,
            "provider_returned_no_output_text".into(),
        ))
    } else {
        Ok(chunks.join(""))
    }
}

fn field_value<'a>(output: &'a ProbeModelOutput, field: &str) -> Option<&'a String> {
    match field {
        "primary_task" => output.primary_task.as_ref(),
        "current_step" => output.current_step.as_ref(),
        "last_progress" => output.last_progress.as_ref(),
        "unfinished_state" => output.unfinished_state.as_ref(),
        _ => None,
    }
}

fn set_field_value(output: &mut ProbeModelOutput, field: &str, value: Option<String>) {
    match field {
        "primary_task" => output.primary_task = value,
        "current_step" => output.current_step = value,
        "last_progress" => output.last_progress = value,
        "unfinished_state" => output.unfinished_state = value,
        _ => {}
    }
}

fn primary_task_is_generic(value: &str) -> bool {
    let normalized = value
        .trim()
        .trim_matches(|character: char| !character.is_alphanumeric() && character != '_')
        .to_ascii_lowercase();
    [
        "editing",
        "viewing",
        "browsing",
        "reviewing",
        "reviewing_output",
        "reviewing output",
        "typing",
        "filling_form",
        "filling form",
        "searching",
    ]
    .iter()
    .any(|generic| {
        normalized == *generic
            || normalized.starts_with(&format!("{generic} "))
            || normalized.starts_with(&format!("{generic} a "))
            || normalized.starts_with(&format!("{generic} the "))
    })
}

fn source_fingerprint_matches(packet: &ObservationPacketV2, slot: &SupportSlot) -> bool {
    match slot.source_kind.as_str() {
        "keyframe" => packet
            .semantic_keyframes
            .iter()
            .chain(std::iter::once(&packet.current_frame))
            .find(|frame| frame.frame_id == slot.record_id)
            .is_some_and(|frame| {
                fingerprint(frame) == slot.source_fingerprint
                    && frame.local_image_handle_hash == slot.content_hash
                    && frame.model_eligible
                    && !is_private_status(Some(&frame.privacy_status))
            }),
        "surface_identity" => packet
            .semantic_keyframes
            .iter()
            .chain(std::iter::once(&packet.current_frame))
            .find(|frame| frame.frame_id == slot.record_id)
            .is_some_and(|frame| fingerprint(&frame.surface_identity) == slot.source_fingerprint),
        "canonical_element" => packet
            .canonical_elements
            .iter()
            .find(|element| element.element_id == slot.record_id)
            .is_some_and(|element| {
                fingerprint(element) == slot.source_fingerprint
                    && element.text_reference == slot.content_hash
                    && element_is_probe_eligible(element)
            }),
        "causal_event" => packet
            .causal_events
            .iter()
            .find(|event| event.event_id == slot.record_id)
            .is_some_and(|event| fingerprint(event) == slot.source_fingerprint),
        "semantic_delta" => packet
            .frame_changes
            .iter()
            .find(|delta| delta.delta_id == slot.record_id)
            .is_some_and(|delta| {
                fingerprint(delta) == slot.source_fingerprint
                    && delta.summary_hash == slot.content_hash
            }),
        _ => false,
    }
}

pub(crate) fn admit_output(
    packet: &ObservationPacketV2,
    request: &ProbeRequest,
    mut output: ProbeModelOutput,
) -> (ProbeModelOutput, Vec<String>) {
    let mut issues = Vec::new();
    let expected_fields = SEMANTIC_FIELDS.into_iter().collect::<BTreeSet<_>>();
    let actual_support_fields = output
        .support_slots_by_field
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let actual_confidence_fields = output
        .confidence_by_field
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if actual_support_fields != expected_fields {
        issues.push("support_field_set_mismatch".into());
    }
    if actual_confidence_fields != expected_fields {
        issues.push("confidence_field_set_mismatch".into());
    }

    for field in SEMANTIC_FIELDS {
        let value = field_value(&output, field).cloned();
        let supports = output
            .support_slots_by_field
            .get(field)
            .cloned()
            .unwrap_or_default();
        let confidence = output
            .confidence_by_field
            .get(field)
            .copied()
            .unwrap_or_default();
        let mut field_issues = Vec::new();
        if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
            field_issues.push("invalid_confidence");
        }
        if value.is_none() {
            if !supports.is_empty() {
                field_issues.push("null_field_has_support_slots");
            }
        } else {
            if supports.is_empty() {
                field_issues.push("non_null_field_has_no_support");
            }
            if field == "primary_task" && value.as_deref().is_some_and(primary_task_is_generic) {
                field_issues.push("forbidden_generic_primary_task");
            }
            for support in &supports {
                match request.slots.get(support) {
                    None => field_issues.push("foreign_or_missing_slot"),
                    Some(slot)
                        if !slot.privacy_eligible
                            || !slot.ownership_eligible
                            || !source_fingerprint_matches(packet, slot) =>
                    {
                        field_issues.push("stale_or_ineligible_slot")
                    }
                    Some(slot) if !semantic_support_allowed(field, slot.category) => {
                        field_issues.push("slot_category_not_allowed_for_field")
                    }
                    Some(_) => {}
                }
            }
        }
        if !field_issues.is_empty() {
            field_issues.sort_unstable();
            field_issues.dedup();
            issues.extend(
                field_issues
                    .iter()
                    .map(|reason| format!("{field}:{reason}")),
            );
            set_field_value(&mut output, field, None);
            output
                .support_slots_by_field
                .insert(field.into(), Vec::new());
            output.confidence_by_field.insert(field.into(), 0.0);
        } else if let Some(value) = value {
            set_field_value(&mut output, field, Some(bounded_text(&value, 320)));
        }
    }
    let admitted_count = SEMANTIC_FIELDS
        .iter()
        .filter(|field| field_value(&output, field).is_some())
        .count();
    output.status = if admitted_count == 0 {
        ProbeResolutionStatus::Unresolved
    } else if admitted_count == SEMANTIC_FIELDS.len()
        && output.primary_task.is_some()
        && issues.is_empty()
    {
        ProbeResolutionStatus::Resolved
    } else {
        ProbeResolutionStatus::PartlyResolved
    };
    output.missing_evidence = output
        .missing_evidence
        .into_iter()
        .take(8)
        .map(|note| bounded_text(&note, 240))
        .collect();
    issues.sort();
    issues.dedup();
    (output, issues)
}

fn parse_probe_response(
    packet: &ObservationPacketV2,
    request: &ProbeRequest,
    response: &Value,
) -> Result<
    (
        ProbeModelOutput,
        usize,
        Vec<String>,
        BTreeMap<String, Vec<String>>,
    ),
    (ProbeDiagnosticStatus, String),
> {
    let text = output_text(response)?;
    let output_bytes = text.len();
    let output = serde_json::from_str::<ProbeModelOutput>(&text).map_err(|_| {
        (
            ProbeDiagnosticStatus::StructuredParseFailure,
            "invalid_probe_structured_output".into(),
        )
    })?;
    let cited_support_slots_before_admission = output.support_slots_by_field.clone();
    let (admitted, issues) = admit_output(packet, request, output);
    Ok((
        admitted,
        output_bytes,
        issues,
        cited_support_slots_before_admission,
    ))
}

fn classify_transport_failure(error: &str) -> (ProbeDiagnosticStatus, String) {
    let normalized = error.to_ascii_lowercase();
    if normalized.contains("timed out") || normalized.contains("timeout") {
        (ProbeDiagnosticStatus::Timeout, "provider_timeout".into())
    } else if normalized.contains("400")
        || normalized.contains("401")
        || normalized.contains("403")
        || normalized.contains("invalid_request")
    {
        (
            ProbeDiagnosticStatus::ProviderRejected,
            "provider_rejected_request".into(),
        )
    } else if normalized.contains("404") || normalized.contains("model") {
        (
            ProbeDiagnosticStatus::ProviderUnavailable,
            "provider_model_unavailable".into(),
        )
    } else {
        (
            ProbeDiagnosticStatus::ProviderUnavailable,
            "provider_transport_error".into(),
        )
    }
}

fn estimated_cost(model_name: &str, usage: &ProviderUsageV1) -> Option<f64> {
    let normalized = model_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    let input_key = format!("SMALLTALK_PFTU_{normalized}_INPUT_USD_PER_MILLION");
    let output_key = format!("SMALLTALK_PFTU_{normalized}_OUTPUT_USD_PER_MILLION");
    let generic_input = "SMALLTALK_PFTU_INPUT_USD_PER_MILLION";
    let generic_output = "SMALLTALK_PFTU_OUTPUT_USD_PER_MILLION";
    let read_rate = |specific: &str, generic: &str| {
        std::env::var(specific)
            .ok()
            .or_else(|| std::env::var(generic).ok())
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite() && *value >= 0.0)
    };
    // Rechecked against the official model pages on 2026-07-13. Exact
    // environment overrides take precedence so a later proof run cannot
    // silently use a stale price.
    let documented_rates = match model_name {
        "gpt-5.6-luna" => Some((1.0, 6.0)),
        "gpt-5.6-sol" => Some((5.0, 30.0)),
        _ => None,
    };
    let input_rate =
        read_rate(&input_key, generic_input).or_else(|| documented_rates.map(|rates| rates.0))?;
    let output_rate =
        read_rate(&output_key, generic_output).or_else(|| documented_rates.map(|rates| rates.1))?;
    Some(
        usage.input_tokens.unwrap_or_default().max(0) as f64 * input_rate / 1_000_000.0
            + usage.output_tokens.unwrap_or_default().max(0) as f64 * output_rate / 1_000_000.0,
    )
}

pub(crate) fn run_probe(
    packet: &ObservationPacketV2,
    model_name: &str,
    api_key: Option<&str>,
) -> (ProbeAttempt, Option<BTreeMap<String, SupportSlot>>) {
    let started = Instant::now();
    let request = match build_probe_request(packet, model_name) {
        Ok(request) => request,
        Err((diagnostic_status, failure_reason)) => {
            return (
                ProbeAttempt {
                    diagnostic_status,
                    model: model_name.into(),
                    request_id: None,
                    provider_request_id: None,
                    response_id: None,
                    response_model: None,
                    request_audit: None,
                    usage: ProviderUsageV1::default(),
                    estimated_cost_usd: None,
                    latency_ms: started.elapsed().as_millis() as i64,
                    output_bytes: None,
                    parsed_response: false,
                    cited_support_slots_before_admission: BTreeMap::new(),
                    admitted_output: None,
                    validation_issues: Vec::new(),
                    failure_reason: Some(failure_reason),
                },
                None,
            );
        }
    };
    let slots = request.slots.clone();
    let Some(api_key) = api_key.filter(|value| !value.trim().is_empty()) else {
        return (
            ProbeAttempt {
                diagnostic_status: ProbeDiagnosticStatus::ProviderUnavailable,
                model: model_name.into(),
                request_id: Some(request.audit.request_id.clone()),
                provider_request_id: None,
                response_id: None,
                response_model: None,
                request_audit: Some(request.audit),
                usage: ProviderUsageV1::default(),
                estimated_cost_usd: None,
                latency_ms: started.elapsed().as_millis() as i64,
                output_bytes: None,
                parsed_response: false,
                cited_support_slots_before_admission: BTreeMap::new(),
                admitted_output: None,
                validation_issues: Vec::new(),
                failure_reason: Some("credentials_missing".into()),
            },
            Some(slots),
        );
    };
    let response =
        match super::super::call_openai_responses_with_timeout(api_key, &request.body, 90, 1) {
            Ok(response) => response,
            Err(error) => {
                let (diagnostic_status, failure_reason) = classify_transport_failure(&error);
                return (
                    ProbeAttempt {
                        diagnostic_status,
                        model: model_name.into(),
                        request_id: Some(request.audit.request_id.clone()),
                        provider_request_id: None,
                        response_id: None,
                        response_model: None,
                        request_audit: Some(request.audit),
                        usage: ProviderUsageV1::default(),
                        estimated_cost_usd: None,
                        latency_ms: started.elapsed().as_millis() as i64,
                        output_bytes: None,
                        parsed_response: false,
                        cited_support_slots_before_admission: BTreeMap::new(),
                        admitted_output: None,
                        validation_issues: Vec::new(),
                        failure_reason: Some(failure_reason),
                    },
                    Some(slots),
                );
            }
        };
    let metadata = model::provider_attempt_metadata(&response);
    match parse_probe_response(packet, &request, &response) {
        Ok((
            admitted_output,
            output_bytes,
            validation_issues,
            cited_support_slots_before_admission,
        )) => {
            let diagnostic_status = if validation_issues.is_empty() {
                ProbeDiagnosticStatus::Success
            } else {
                ProbeDiagnosticStatus::SupportSlotValidationFailure
            };
            let estimated_cost_usd = estimated_cost(model_name, &metadata.usage);
            (
                ProbeAttempt {
                    diagnostic_status,
                    model: model_name.into(),
                    request_id: Some(request.audit.request_id.clone()),
                    provider_request_id: metadata.request_id,
                    response_id: metadata.response_id,
                    response_model: metadata.model,
                    request_audit: Some(request.audit),
                    usage: metadata.usage,
                    estimated_cost_usd,
                    latency_ms: started.elapsed().as_millis() as i64,
                    output_bytes: Some(output_bytes),
                    parsed_response: true,
                    cited_support_slots_before_admission,
                    admitted_output: Some(admitted_output),
                    validation_issues,
                    failure_reason: None,
                },
                Some(slots),
            )
        }
        Err((diagnostic_status, failure_reason)) => (
            ProbeAttempt {
                diagnostic_status,
                model: model_name.into(),
                request_id: Some(request.audit.request_id.clone()),
                provider_request_id: metadata.request_id,
                response_id: metadata.response_id,
                response_model: metadata.model,
                request_audit: Some(request.audit),
                usage: metadata.usage,
                estimated_cost_usd: None,
                latency_ms: started.elapsed().as_millis() as i64,
                output_bytes: None,
                parsed_response: false,
                cited_support_slots_before_admission: BTreeMap::new(),
                admitted_output: None,
                validation_issues: Vec::new(),
                failure_reason: Some(failure_reason),
            },
            Some(slots),
        ),
    }
}

fn runtime_or_compiled(value: Option<String>, compiled: Option<&'static str>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty()).or_else(|| {
        compiled
            .map(str::to_string)
            .filter(|value| !value.trim().is_empty())
    })
}

pub(crate) fn configured_model_name() -> String {
    runtime_or_compiled(
        std::env::var("SMALLTALK_PFTU_COST_MODEL").ok(),
        option_env!("SMALLTALK_PFTU_COST_MODEL"),
    )
    .or_else(|| {
        runtime_or_compiled(
            std::env::var("SMALLTALK_TASK_TRUTH_MODEL").ok(),
            option_env!("SMALLTALK_TASK_TRUTH_MODEL"),
        )
    })
    .or_else(|| {
        super::super::project_dotenv_values()
            .ok()
            .and_then(|values| {
                values
                    .get("SMALLTALK_PFTU_COST_MODEL")
                    .or_else(|| values.get("SMALLTALK_TASK_TRUTH_MODEL"))
                    .cloned()
            })
    })
    .filter(|value| !value.trim().is_empty())
    .unwrap_or_else(|| DEFAULT_LUNA_MODEL.into())
}

pub(crate) fn configured_case_id() -> Option<String> {
    runtime_or_compiled(
        std::env::var("SMALLTALK_PFTU_CASE_ID").ok(),
        option_env!("SMALLTALK_PFTU_CASE_ID"),
    )
}

pub(crate) fn probe_enabled() -> bool {
    runtime_or_compiled(
        std::env::var("SMALLTALK_PFTU_SEMANTIC_PROBE_ENABLED").ok(),
        option_env!("SMALLTALK_PFTU_SEMANTIC_PROBE_ENABLED"),
    )
    .as_deref()
    .is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on" | "enabled"
        )
    })
}

pub(crate) fn ensure_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS task_truth_v2_semantic_probe_cases (
           case_id TEXT PRIMARY KEY,
           case_kind TEXT NOT NULL,
           held_back INTEGER NOT NULL,
           expected_recorded_at_ms INTEGER NOT NULL,
           expected_json TEXT NOT NULL,
           consumed_decision_id TEXT,
           consumed_at_ms INTEGER,
           created_at_ms INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS task_truth_v2_semantic_probe_runs (
           run_id TEXT PRIMARY KEY,
           case_id TEXT NOT NULL,
           decision_id TEXT NOT NULL,
           session_id TEXT,
           packet_id TEXT NOT NULL,
           model TEXT NOT NULL,
           diagnostic_status TEXT NOT NULL,
           request_id TEXT,
           provider_request_id TEXT,
           response_id TEXT,
           response_model TEXT,
           request_audit_json TEXT,
           support_slot_map_json TEXT,
           cited_support_slots_json TEXT NOT NULL DEFAULT '{}',
           admitted_output_json TEXT,
           validation_issues_json TEXT NOT NULL,
           failure_reason TEXT,
           input_tokens INTEGER,
           output_tokens INTEGER,
           total_tokens INTEGER,
           estimated_cost_usd REAL,
           latency_ms INTEGER NOT NULL,
           output_bytes INTEGER,
           parsed_response INTEGER NOT NULL,
           created_at_ms INTEGER NOT NULL,
           FOREIGN KEY(case_id) REFERENCES task_truth_v2_semantic_probe_cases(case_id)
         );
         CREATE INDEX IF NOT EXISTS idx_task_truth_v2_semantic_probe_runs_case
           ON task_truth_v2_semantic_probe_runs(case_id, created_at_ms, model);",
    )
    .map_err(|error| error.to_string())?;
    let has_cited_support_column = conn
        .prepare("PRAGMA table_info(task_truth_v2_semantic_probe_runs)")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|error| error.to_string())?
        .iter()
        .any(|column| column == "cited_support_slots_json");
    if !has_cited_support_column {
        conn.execute(
            "ALTER TABLE task_truth_v2_semantic_probe_runs
             ADD COLUMN cited_support_slots_json TEXT NOT NULL DEFAULT '{}'",
            [],
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn validate_armed_case(case: &ArmedProbeCase) -> Result<(), String> {
    if case.case_id.trim().is_empty() || case.case_kind.trim().is_empty() {
        return Err("probe_case_identity_missing".into());
    }
    if case.expected_recorded_at_ms <= 0 {
        return Err("probe_case_expected_timestamp_missing".into());
    }
    let fields = case
        .recoverable_by_field
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if fields != SEMANTIC_FIELDS.into_iter().collect::<BTreeSet<_>>() {
        return Err("probe_case_recoverable_field_set_mismatch".into());
    }
    for field in SEMANTIC_FIELDS {
        let expected = match field {
            "primary_task" => case.expected_primary_task.as_ref(),
            "current_step" => case.expected_current_step.as_ref(),
            "last_progress" => case.expected_last_progress.as_ref(),
            "unfinished_state" => case.expected_unfinished_state.as_ref(),
            _ => None,
        };
        if case.recoverable_by_field.get(field) == Some(&true)
            && expected.is_none_or(|value| value.trim().is_empty())
        {
            return Err(format!(
                "probe_case_recoverable_expected_value_missing:{field}"
            ));
        }
    }
    Ok(())
}

pub(crate) fn arm_case(conn: &Connection, case: &ArmedProbeCase) -> Result<(), String> {
    ensure_schema(conn)?;
    validate_armed_case(case)?;
    let existing_runs: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM task_truth_v2_semantic_probe_runs WHERE case_id=?1",
            [case.case_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if existing_runs > 0 {
        return Err("probe_case_cannot_be_rearmed_after_output".into());
    }
    let now_ms = super::super::current_time_millis();
    if case.expected_recorded_at_ms > now_ms {
        return Err("probe_case_expected_timestamp_is_in_future".into());
    }
    let expected_json = serde_json::to_string(case).map_err(|error| error.to_string())?;
    conn.execute(
        "INSERT INTO task_truth_v2_semantic_probe_cases (
           case_id, case_kind, held_back, expected_recorded_at_ms, expected_json,
           consumed_decision_id, consumed_at_ms, created_at_ms
         ) VALUES (?1,?2,?3,?4,?5,NULL,NULL,?6)
         ON CONFLICT(case_id) DO UPDATE SET
           case_kind=excluded.case_kind,
           held_back=excluded.held_back,
           expected_recorded_at_ms=excluded.expected_recorded_at_ms,
           expected_json=excluded.expected_json,
           consumed_decision_id=NULL,
           consumed_at_ms=NULL,
           created_at_ms=excluded.created_at_ms",
        params![
            case.case_id,
            case.case_kind,
            i64::from(case.held_back),
            case.expected_recorded_at_ms,
            expected_json,
            now_ms,
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn load_armed_case(conn: &Connection, case_id: &str) -> Result<ArmedProbeCase, String> {
    let raw = conn
        .query_row(
            "SELECT expected_json FROM task_truth_v2_semantic_probe_cases
             WHERE case_id=?1 AND consumed_decision_id IS NULL",
            [case_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|_| "probe_case_not_armed_or_already_consumed".to_string())?;
    let case = serde_json::from_str::<ArmedProbeCase>(&raw)
        .map_err(|_| "probe_case_expected_json_invalid".to_string())?;
    validate_armed_case(&case)?;
    Ok(case)
}

fn validate_expected_timing(expected_recorded_at_ms: i64, now_ms: i64) -> Result<(), String> {
    if expected_recorded_at_ms >= now_ms {
        return Err("probe_expected_meaning_was_not_recorded_before_output".into());
    }
    if now_ms.saturating_sub(expected_recorded_at_ms) > MAX_ARMED_CASE_AGE_MS {
        return Err("probe_expected_meaning_is_stale".into());
    }
    Ok(())
}

fn diagnostic_label(status: ProbeDiagnosticStatus) -> &'static str {
    match status {
        ProbeDiagnosticStatus::RequestNotBuilt => "request_not_built",
        ProbeDiagnosticStatus::PrivacyBlocked => "privacy_blocked",
        ProbeDiagnosticStatus::ProviderRejected => "provider_rejected",
        ProbeDiagnosticStatus::ProviderUnavailable => "provider_unavailable",
        ProbeDiagnosticStatus::Timeout => "timeout",
        ProbeDiagnosticStatus::ProviderNoUsableOutput => "provider_no_usable_output",
        ProbeDiagnosticStatus::StructuredParseFailure => "structured_parse_failure",
        ProbeDiagnosticStatus::SupportSlotValidationFailure => "support_slot_validation_failure",
        ProbeDiagnosticStatus::HumanRatedWrong => "human_rated_wrong",
        ProbeDiagnosticStatus::Success => "success",
    }
}

fn persist_attempt(
    conn: &Connection,
    case_id: &str,
    decision_id: &str,
    session_id: Option<&str>,
    packet: &ObservationPacketV2,
    attempt: &ProbeAttempt,
    slots: Option<&BTreeMap<String, SupportSlot>>,
) -> Result<(), String> {
    let created_at_ms = super::super::current_time_millis();
    let run_id = format!(
        "pftu-probe-run-{}",
        super::super::stable_hash(
            format!(
                "{case_id}:{decision_id}:{}:{}:{created_at_ms}",
                packet.packet_id, attempt.model
            )
            .as_bytes()
        )
    );
    conn.execute(
        "INSERT INTO task_truth_v2_semantic_probe_runs (
           run_id, case_id, decision_id, session_id, packet_id, model,
           diagnostic_status, request_id, provider_request_id, response_id,
           response_model, request_audit_json, support_slot_map_json,
           cited_support_slots_json, admitted_output_json, validation_issues_json, failure_reason,
           input_tokens, output_tokens, total_tokens, estimated_cost_usd,
           latency_ms, output_bytes, parsed_response, created_at_ms
         ) VALUES (
           ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,
           ?17,?18,?19,?20,?21,?22,?23,?24,?25
         )",
        params![
            run_id,
            case_id,
            decision_id,
            session_id,
            packet.packet_id,
            attempt.model,
            diagnostic_label(attempt.diagnostic_status),
            attempt.request_id,
            attempt.provider_request_id,
            attempt.response_id,
            attempt.response_model,
            attempt
                .request_audit
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|error| error.to_string())?,
            slots
                .map(serde_json::to_string)
                .transpose()
                .map_err(|error| error.to_string())?,
            serde_json::to_string(&attempt.cited_support_slots_before_admission)
                .map_err(|error| error.to_string())?,
            attempt
                .admitted_output
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|error| error.to_string())?,
            serde_json::to_string(&attempt.validation_issues).map_err(|error| error.to_string())?,
            attempt.failure_reason,
            attempt.usage.input_tokens,
            attempt.usage.output_tokens,
            attempt.usage.total_tokens,
            attempt.estimated_cost_usd,
            attempt.latency_ms,
            attempt.output_bytes.map(|value| value as i64),
            i64::from(attempt.parsed_response),
            created_at_ms,
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn configured_model() -> Result<(Option<String>, String), String> {
    let mut config = super::super::continue_openai_config(None)?;
    config.model = configured_model_name();
    Ok((config.api_key, config.model))
}

pub(crate) fn run_manual_probe(
    conn: &Connection,
    decision_id: &str,
    session_id: Option<&str>,
    packet: &ObservationPacketV2,
) -> Result<(), String> {
    ensure_schema(conn)?;
    let case_id = configured_case_id().ok_or_else(|| "probe_case_id_not_configured".to_string())?;
    let armed = load_armed_case(conn, &case_id)?;
    let now_ms = super::super::current_time_millis();
    validate_expected_timing(armed.expected_recorded_at_ms, now_ms)?;
    let (api_key, model_name) = configured_model()?;
    let (attempt, slots) = run_probe(packet, &model_name, api_key.as_deref());
    persist_attempt(
        conn,
        &case_id,
        decision_id,
        session_id,
        packet,
        &attempt,
        slots.as_ref(),
    )?;
    conn.execute(
        "UPDATE task_truth_v2_semantic_probe_cases
         SET consumed_decision_id=?1, consumed_at_ms=?2
         WHERE case_id=?3 AND consumed_decision_id IS NULL",
        params![decision_id, now_ms, case_id],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FieldJudgment {
    Correct,
    PartlyRight,
    Wrong,
    ShouldBeUnresolved,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProofAttempt {
    pub(crate) model: String,
    pub(crate) real_provider_round_trip: bool,
    pub(crate) diagnostic_status: ProbeDiagnosticStatus,
    pub(crate) parsed_response: bool,
    pub(crate) provider_request_id_present: bool,
    pub(crate) response_id_present: bool,
    pub(crate) response_recorded_at_ms: i64,
    pub(crate) request_bytes: usize,
    pub(crate) request_estimated_tokens: usize,
    pub(crate) image_count: usize,
    pub(crate) output_bytes: usize,
    pub(crate) input_tokens: Option<i64>,
    pub(crate) output_tokens: Option<i64>,
    pub(crate) estimated_cost_usd: Option<f64>,
    pub(crate) latency_ms: i64,
    pub(crate) output_status: ProbeResolutionStatus,
    pub(crate) output_by_field: BTreeMap<String, Option<String>>,
    pub(crate) confidence_by_field: BTreeMap<String, f64>,
    pub(crate) cited_support_slots_by_field: BTreeMap<String, Vec<String>>,
    pub(crate) support_admitted_by_field: BTreeMap<String, bool>,
    pub(crate) unsupported_fields_null_or_rejected: bool,
    pub(crate) local_semantic_fallback_used: bool,
    pub(crate) judgments_by_field: BTreeMap<String, FieldJudgment>,
    pub(crate) corrections_by_field: BTreeMap<String, String>,
    pub(crate) concrete_without_app_or_generic_verb: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProofCase {
    pub(crate) case_id: String,
    pub(crate) case_kind: String,
    pub(crate) held_back: bool,
    pub(crate) case_timestamp_ms: i64,
    pub(crate) session_id: String,
    pub(crate) decision_id: String,
    pub(crate) expected_recorded_at_ms: i64,
    pub(crate) expected_by_field: BTreeMap<String, Option<String>>,
    pub(crate) recoverable_by_field: BTreeMap<String, bool>,
    pub(crate) human_reviewed: bool,
    pub(crate) reviewer_id: String,
    pub(crate) attempts: Vec<ProofAttempt>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct OldRequestMeasurement {
    pub(crate) source_session: String,
    pub(crate) structured_bytes: usize,
    pub(crate) estimated_tokens: usize,
    pub(crate) image_count: usize,
    pub(crate) max_output_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProofCorpus {
    pub(crate) schema: String,
    pub(crate) frozen_before_holdout: bool,
    pub(crate) current_model: String,
    pub(crate) chosen_model: String,
    pub(crate) old_request: OldRequestMeasurement,
    pub(crate) cases: Vec<ProofCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ProofGateMetrics {
    pub(crate) case_count: usize,
    pub(crate) held_back_case_count: usize,
    pub(crate) real_round_trip_and_parse_count: usize,
    pub(crate) confident_wrong_primary_task_count: usize,
    pub(crate) recoverable_field_count: usize,
    pub(crate) correct_or_partly_recoverable_field_count: usize,
    pub(crate) recoverable_field_quality: f64,
    pub(crate) recoverable_primary_task_count: usize,
    pub(crate) correct_recoverable_primary_task_count: usize,
    pub(crate) recoverable_primary_task_accuracy: f64,
    pub(crate) held_back_recoverable_primary_task_count: usize,
    pub(crate) held_back_correct_primary_task_count: usize,
    pub(crate) held_back_primary_task_accuracy: f64,
    pub(crate) understandable_answer_count: usize,
    pub(crate) largest_new_request_bytes: usize,
    pub(crate) largest_new_request_tokens: usize,
    pub(crate) largest_new_image_count: usize,
    pub(crate) largest_new_output_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ProofGateReport {
    pub(crate) schema: String,
    pub(crate) passed: bool,
    pub(crate) chosen_model: String,
    pub(crate) metrics: ProofGateMetrics,
    pub(crate) violations: Vec<String>,
}

const REQUIRED_CASE_KINDS: [&str; 12] = [
    "named_product_code_change",
    "command_verification",
    "agent_response_review",
    "browser_research_support",
    "api_dashboard_support",
    "task_indeterminable",
    "completed_without_unfinished_state",
    "waiting_for_agent_or_command",
    "form_business_purpose_visible",
    "form_activity_only_visible",
    "session_038_reconstruction",
    "previously_unseen_application",
];

fn exact_semantic_field_set<T>(map: &BTreeMap<String, T>) -> bool {
    map.keys().map(String::as_str).collect::<BTreeSet<_>>()
        == SEMANTIC_FIELDS.into_iter().collect::<BTreeSet<_>>()
}

fn chosen_attempt<'a>(case: &'a ProofCase, model_name: &str) -> Result<&'a ProofAttempt, String> {
    let attempts = case
        .attempts
        .iter()
        .filter(|attempt| attempt.model == model_name)
        .collect::<Vec<_>>();
    if attempts.len() == 1 {
        Ok(attempts[0])
    } else {
        Err(format!(
            "{}:chosen_model_attempt_count={}",
            case.case_id,
            attempts.len()
        ))
    }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

pub(crate) fn evaluate_proof_corpus(corpus: &ProofCorpus) -> ProofGateReport {
    let mut violations = Vec::new();
    if corpus.schema != PROBE_CORPUS_SCHEMA {
        violations.push("corpus_schema_mismatch".into());
    }
    if !corpus.frozen_before_holdout {
        violations.push("corpus_not_frozen_before_holdout".into());
    }
    if corpus.current_model.trim().is_empty() {
        violations.push("current_model_missing".into());
    }
    if corpus.chosen_model != corpus.current_model {
        violations.push("chosen_model_must_equal_current_model".into());
    }
    if corpus.cases.len() != 12 {
        violations.push(format!(
            "case_count_must_equal_12:actual={}",
            corpus.cases.len()
        ));
    }
    let kinds = corpus
        .cases
        .iter()
        .map(|case| case.case_kind.as_str())
        .collect::<BTreeSet<_>>();
    for required in REQUIRED_CASE_KINDS {
        if !kinds.contains(required) {
            violations.push(format!("required_case_kind_missing:{required}"));
        }
    }
    let held_back_case_count = corpus.cases.iter().filter(|case| case.held_back).count();
    if held_back_case_count < 4 {
        violations.push(format!(
            "held_back_case_count_below_four:actual={held_back_case_count}"
        ));
    }

    let mut real_round_trip_and_parse_count = 0usize;
    let mut confident_wrong_primary_task_count = 0usize;
    let mut recoverable_field_count = 0usize;
    let mut correct_or_partly_recoverable_field_count = 0usize;
    let mut recoverable_primary_task_count = 0usize;
    let mut correct_recoverable_primary_task_count = 0usize;
    let mut held_back_recoverable_primary_task_count = 0usize;
    let mut held_back_correct_primary_task_count = 0usize;
    let mut understandable_answer_count = 0usize;
    let mut largest_new_request_bytes = 0usize;
    let mut largest_new_request_tokens = 0usize;
    let mut largest_new_image_count = 0usize;
    let mut largest_new_output_bytes = 0usize;

    for case in &corpus.cases {
        if case.case_id.trim().is_empty()
            || case.session_id.trim().is_empty()
            || case.decision_id.trim().is_empty()
            || case.case_timestamp_ms <= 0
            || case.expected_recorded_at_ms <= 0
        {
            violations.push(format!(
                "{}:case_identity_or_timestamp_missing",
                case.case_id
            ));
        }
        if !case.human_reviewed || case.reviewer_id.trim().is_empty() {
            violations.push(format!("{}:human_review_missing", case.case_id));
        }
        if !exact_semantic_field_set(&case.expected_by_field)
            || !exact_semantic_field_set(&case.recoverable_by_field)
        {
            violations.push(format!("{}:expected_field_set_mismatch", case.case_id));
        }
        let models = case
            .attempts
            .iter()
            .map(|attempt| attempt.model.as_str())
            .collect::<BTreeSet<_>>();
        if models.len() != 1 || !models.contains(corpus.current_model.as_str()) {
            violations.push(format!(
                "{}:single_current_model_attempt_required",
                case.case_id
            ));
        }
        for attempt in &case.attempts {
            if attempt.response_recorded_at_ms <= case.expected_recorded_at_ms {
                violations.push(format!(
                    "{}:{}:expected_not_recorded_before_output",
                    case.case_id, attempt.model
                ));
            }
            if !exact_semantic_field_set(&attempt.output_by_field)
                || !exact_semantic_field_set(&attempt.confidence_by_field)
                || !exact_semantic_field_set(&attempt.cited_support_slots_by_field)
                || !exact_semantic_field_set(&attempt.support_admitted_by_field)
                || !exact_semantic_field_set(&attempt.judgments_by_field)
                || !exact_semantic_field_set(&attempt.corrections_by_field)
            {
                violations.push(format!(
                    "{}:{}:attempt_field_set_mismatch",
                    case.case_id, attempt.model
                ));
            }
            if attempt.real_provider_round_trip
                && (!attempt.provider_request_id_present || !attempt.response_id_present)
            {
                violations.push(format!(
                    "{}:{}:provider_identity_missing",
                    case.case_id, attempt.model
                ));
            }
            if attempt.estimated_cost_usd.is_none() {
                violations.push(format!(
                    "{}:{}:estimated_cost_missing",
                    case.case_id, attempt.model
                ));
            }
        }

        let attempt = match chosen_attempt(case, &corpus.chosen_model) {
            Ok(attempt) => attempt,
            Err(error) => {
                violations.push(error);
                continue;
            }
        };
        if attempt.real_provider_round_trip && attempt.parsed_response {
            real_round_trip_and_parse_count += 1;
        }
        largest_new_request_bytes = largest_new_request_bytes.max(attempt.request_bytes);
        largest_new_request_tokens =
            largest_new_request_tokens.max(attempt.request_estimated_tokens);
        largest_new_image_count = largest_new_image_count.max(attempt.image_count);
        largest_new_output_bytes = largest_new_output_bytes.max(attempt.output_bytes);
        if attempt.concrete_without_app_or_generic_verb {
            understandable_answer_count += 1;
        }
        if attempt.local_semantic_fallback_used {
            violations.push(format!("{}:local_semantic_fallback_used", case.case_id));
        }
        if !attempt.unsupported_fields_null_or_rejected {
            violations.push(format!(
                "{}:unsupported_field_was_not_null_or_rejected",
                case.case_id
            ));
        }
        for field in SEMANTIC_FIELDS {
            let recoverable = case.recoverable_by_field.get(field) == Some(&true);
            let judgment = attempt.judgments_by_field.get(field);
            let output_value = attempt
                .output_by_field
                .get(field)
                .and_then(|value| value.as_ref());
            let support_slots = attempt
                .cited_support_slots_by_field
                .get(field)
                .cloned()
                .unwrap_or_default();
            let support_admitted = attempt.support_admitted_by_field.get(field) == Some(&true);
            if output_value.is_some() && (support_slots.is_empty() || !support_admitted) {
                violations.push(format!(
                    "{}:{field}:nonnull_field_without_admitted_support",
                    case.case_id
                ));
            }
            if recoverable {
                recoverable_field_count += 1;
                if matches!(
                    judgment,
                    Some(FieldJudgment::Correct | FieldJudgment::PartlyRight)
                ) {
                    correct_or_partly_recoverable_field_count += 1;
                }
            }
            if field == "primary_task" && recoverable {
                recoverable_primary_task_count += 1;
                if judgment == Some(&FieldJudgment::Correct) {
                    correct_recoverable_primary_task_count += 1;
                }
                if case.held_back {
                    held_back_recoverable_primary_task_count += 1;
                    if judgment == Some(&FieldJudgment::Correct) {
                        held_back_correct_primary_task_count += 1;
                    }
                }
            }
        }
        let primary_wrong =
            attempt.judgments_by_field.get("primary_task") == Some(&FieldJudgment::Wrong);
        let primary_confidence = attempt
            .confidence_by_field
            .get("primary_task")
            .copied()
            .unwrap_or_default();
        if primary_wrong && primary_confidence >= 0.7 {
            confident_wrong_primary_task_count += 1;
            violations.push(format!("{}:confident_wrong_primary_task", case.case_id));
        }
        if case.held_back && primary_wrong && primary_confidence >= 0.7 {
            violations.push(format!(
                "{}:held_back_confident_wrong_primary_task",
                case.case_id
            ));
        }
    }

    let recoverable_field_quality = ratio(
        correct_or_partly_recoverable_field_count,
        recoverable_field_count,
    );
    let recoverable_primary_task_accuracy = ratio(
        correct_recoverable_primary_task_count,
        recoverable_primary_task_count,
    );
    let held_back_primary_task_accuracy = ratio(
        held_back_correct_primary_task_count,
        held_back_recoverable_primary_task_count,
    );
    if real_round_trip_and_parse_count < 10 {
        violations.push(format!(
            "real_round_trip_and_parse_below_10:actual={real_round_trip_and_parse_count}"
        ));
    }
    if recoverable_field_quality < 0.9 {
        violations.push(format!(
            "recoverable_field_quality_below_0_90:actual={recoverable_field_quality:.4}"
        ));
    }
    if recoverable_primary_task_accuracy < 0.8 {
        violations.push(format!(
            "recoverable_primary_task_accuracy_below_0_80:actual={recoverable_primary_task_accuracy:.4}"
        ));
    }
    if held_back_primary_task_accuracy < 0.75 {
        violations.push(format!(
            "held_back_primary_task_accuracy_below_0_75:actual={held_back_primary_task_accuracy:.4}"
        ));
    }
    if understandable_answer_count < 10 {
        violations.push(format!(
            "understandable_answer_count_below_10:actual={understandable_answer_count}"
        ));
    }
    if largest_new_request_bytes >= corpus.old_request.structured_bytes
        || largest_new_request_tokens >= corpus.old_request.estimated_tokens
    {
        violations.push("new_request_not_materially_smaller_than_session_038".into());
    }
    if largest_new_request_bytes * 2 >= corpus.old_request.structured_bytes {
        violations.push("new_request_bytes_not_at_least_50_percent_smaller".into());
    }
    if largest_new_image_count > MAX_IMAGES {
        violations.push("new_request_image_count_exceeds_four".into());
    }

    violations.sort();
    violations.dedup();
    ProofGateReport {
        schema: "smalltalk.pftu_01.proof_gate_report.v1".into(),
        passed: violations.is_empty(),
        chosen_model: corpus.chosen_model.clone(),
        metrics: ProofGateMetrics {
            case_count: corpus.cases.len(),
            held_back_case_count,
            real_round_trip_and_parse_count,
            confident_wrong_primary_task_count,
            recoverable_field_count,
            correct_or_partly_recoverable_field_count,
            recoverable_field_quality,
            recoverable_primary_task_count,
            correct_recoverable_primary_task_count,
            recoverable_primary_task_accuracy,
            held_back_recoverable_primary_task_count,
            held_back_correct_primary_task_count,
            held_back_primary_task_accuracy,
            understandable_answer_count,
            largest_new_request_bytes,
            largest_new_request_tokens,
            largest_new_image_count,
            largest_new_output_bytes,
        },
        violations,
    }
}

pub(crate) fn arm_case_from_path(
    database: &std::path::Path,
    input: &std::path::Path,
) -> Result<Value, String> {
    let case = serde_json::from_slice::<ArmedProbeCase>(
        &std::fs::read(input).map_err(|error| format!("{}: {error}", input.display()))?,
    )
    .map_err(|error| format!("{}: {error}", input.display()))?;
    let conn = Connection::open(database).map_err(|error| error.to_string())?;
    arm_case(&conn, &case)?;
    Ok(json!({
        "schema":"smalltalk.pftu_01.arm_result.v1",
        "case_id":case.case_id,
        "expected_recorded_at_ms":case.expected_recorded_at_ms,
        "armed":true
    }))
}

pub(crate) fn evaluate_corpus_path(
    input: &std::path::Path,
    output: Option<&std::path::Path>,
) -> Result<Value, String> {
    let corpus = serde_json::from_slice::<ProofCorpus>(
        &std::fs::read(input).map_err(|error| format!("{}: {error}", input.display()))?,
    )
    .map_err(|error| format!("{}: {error}", input.display()))?;
    let report = evaluate_proof_corpus(&corpus);
    let value = serde_json::to_value(&report).map_err(|error| error.to_string())?;
    if let Some(output) = output {
        let bytes = serde_json::to_vec_pretty(&value).map_err(|error| error.to_string())?;
        std::fs::write(output, bytes).map_err(|error| format!("{}: {error}", output.display()))?;
    }
    Ok(value)
}

pub(crate) fn export_private_review_bundle(
    database: &std::path::Path,
    output: &std::path::Path,
) -> Result<Value, String> {
    let conn = Connection::open(database).map_err(|error| error.to_string())?;
    ensure_schema(&conn)?;
    let mut statement = conn
        .prepare(
            "SELECT c.expected_json, c.consumed_decision_id, c.consumed_at_ms,
                    r.session_id, r.packet_id, r.model, r.diagnostic_status,
                    r.request_id, r.provider_request_id, r.response_id, r.response_model,
                    r.request_audit_json, r.cited_support_slots_json, r.admitted_output_json,
                    r.validation_issues_json, r.failure_reason,
                    r.input_tokens, r.output_tokens, r.total_tokens,
                    r.estimated_cost_usd, r.latency_ms, r.output_bytes,
                    r.parsed_response, r.created_at_ms
             FROM task_truth_v2_semantic_probe_cases c
             LEFT JOIN task_truth_v2_semantic_probe_runs r ON r.case_id=c.case_id
             ORDER BY c.case_id, r.created_at_ms, r.model",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(json!({
                "expected":serde_json::from_str::<Value>(&row.get::<_, String>(0)?).unwrap_or(Value::Null),
                "decision_id":row.get::<_, Option<String>>(1)?,
                "case_consumed_at_ms":row.get::<_, Option<i64>>(2)?,
                "session_id":row.get::<_, Option<String>>(3)?,
                "packet_id":row.get::<_, Option<String>>(4)?,
                "model":row.get::<_, Option<String>>(5)?,
                "diagnostic_status":row.get::<_, Option<String>>(6)?,
                "request_id":row.get::<_, Option<String>>(7)?,
                "provider_request_id":row.get::<_, Option<String>>(8)?,
                "response_id":row.get::<_, Option<String>>(9)?,
                "response_model":row.get::<_, Option<String>>(10)?,
                "request_audit":row.get::<_, Option<String>>(11)?.and_then(|value| serde_json::from_str::<Value>(&value).ok()),
                "cited_support_slots_before_admission":row.get::<_, Option<String>>(12)?.and_then(|value| serde_json::from_str::<Value>(&value).ok()),
                "admitted_output":row.get::<_, Option<String>>(13)?.and_then(|value| serde_json::from_str::<Value>(&value).ok()),
                "validation_issues":row.get::<_, Option<String>>(14)?.and_then(|value| serde_json::from_str::<Value>(&value).ok()),
                "failure_reason":row.get::<_, Option<String>>(15)?,
                "input_tokens":row.get::<_, Option<i64>>(16)?,
                "output_tokens":row.get::<_, Option<i64>>(17)?,
                "total_tokens":row.get::<_, Option<i64>>(18)?,
                "estimated_cost_usd":row.get::<_, Option<f64>>(19)?,
                "latency_ms":row.get::<_, Option<i64>>(20)?,
                "output_bytes":row.get::<_, Option<i64>>(21)?,
                "parsed_response":row.get::<_, Option<i64>>(22)?.map(|value| value != 0),
                "response_recorded_at_ms":row.get::<_, Option<i64>>(23)?,
            }))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let bundle = json!({
        "schema":"smalltalk.pftu_01.private_review_bundle.v1",
        "privacy_warning":"Contains local semantic outputs and expected meanings. Keep outside version control.",
        "rows":rows
    });
    std::fs::write(
        output,
        serde_json::to_vec_pretty(&bundle).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("{}: {error}", output.display()))?;
    Ok(json!({
        "schema":"smalltalk.pftu_01.private_review_export.v1",
        "output":output,
        "row_count":rows.len()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuation::task_truth_v2::observation_packet::{
        ActiveSurfaceIdentityV2, CanonicalElementV2, CausalEventV2, FrameCapacityAccountingV2,
        FrameChangeV2, KeyframeReferenceV2, PacketSizeAccountingV2,
    };

    fn image_file(id: usize) -> String {
        let path = std::env::temp_dir().join(format!(
            "smalltalk-pftu-probe-test-{}-{id}.png",
            std::process::id()
        ));
        std::fs::write(&path, b"probe-image-bytes").expect("write image fixture");
        path.to_string_lossy().into_owned()
    }

    fn surface(id: usize) -> ActiveSurfaceIdentityV2 {
        ActiveSurfaceIdentityV2 {
            app_name: Some(format!("App{id}")),
            app_bundle_id: Some(format!("com.example.app{id}")),
            window_title_hash: Some(format!("window-hash-{id}")),
            window_id: Some(id as i64),
            browser_url_hash: None,
            document_path_hash: Some(format!("document-hash-{id}")),
        }
    }

    fn keyframe(id: usize, private: bool) -> KeyframeReferenceV2 {
        KeyframeReferenceV2 {
            frame_id: format!("frame-{id}"),
            observed_at_ms: id as i64 * 1_000,
            partition: if id == 4 {
                EvidencePartitionV2::Current
            } else {
                EvidencePartitionV2::Prior
            },
            surface_identity: surface(id),
            surface_ownership_confidence: 0.95,
            privacy_status: if private { "private" } else { "allowed" }.into(),
            model_eligible: !private,
            image_source_kind: "native_active_window".into(),
            image_scope: "active_window".into(),
            image_width: Some(100),
            image_height: Some(100),
            image_rejection_reason: private.then(|| "privacy_blocked".into()),
            crop_pixels: None,
            local_image_handle_hash: Some(format!("image-hash-{id}")),
            ephemeral_local_image_path: Some(image_file(id)),
            selection_reasons: vec![if id == 4 {
                "manual_continue_boundary".into()
            } else {
                "material_change_boundary".into()
            }],
        }
    }

    fn element(id: usize) -> CanonicalElementV2 {
        CanonicalElementV2 {
            element_id: format!("element-{id}"),
            frame_id: format!("frame-{id}"),
            bounds: None,
            display_id: Some("display-1".into()),
            window_id: Some(id as i64),
            owning_app_bundle: Some(format!("com.example.app{id}")),
            source_scope: Some("active_window".into()),
            ownership_kind: Some("active_window".into()),
            ownership_confidence: Some(0.95),
            coordinate_space: "window_local".into(),
            freshness: "current".into(),
            text_reference: Some(format!("text-hash-{id}")),
            visual_description: Some(format!(
                "Implement the semantic probe for named product behavior {id}"
            )),
            native_role: Some("AXTextArea".into()),
            native_subrole: None,
            native_actionability: true,
            region_role: RegionRoleV2::ComposerEditor,
            focused: true,
            editable: true,
            selected: false,
            interactive: true,
            parent_element_id: None,
            child_element_ids: Vec::new(),
            source_votes: vec!["accessibility".into()],
            source_conflicts: Vec::new(),
            first_seen_at_ms: id as i64 * 1_000,
            changed_at_ms: id as i64 * 1_000,
            authorship_status: AuthorshipStatusV2::User,
            causal_evidence_refs: Vec::new(),
            task_eligible: true,
            rejection_reasons: Vec::new(),
        }
    }

    fn event(id: usize) -> CausalEventV2 {
        CausalEventV2 {
            event_id: format!("event-{id}"),
            event_kind: "typing_commit".into(),
            observed_at_ms: id as i64 * 1_000 + 100,
            frame_id: format!("frame-{id}"),
            source_frame_id: format!("frame-{id}"),
            target_frame_id: Some(format!("frame-{id}")),
            target_element_id: Some(format!("element-{id}")),
            target_region: Some(RegionRoleV2::ComposerEditor),
            focused_element_before: None,
            focused_element_after: Some(format!("element-{id}")),
            window_id: Some(id as i64),
            app_bundle_id: Some(format!("com.example.app{id}")),
            pointer_x: None,
            pointer_y: None,
            scroll_delta_x: None,
            scroll_delta_y: None,
            pre_state_reference: None,
            post_state_reference: None,
            semantic_delta_reference: Some(format!("delta-{id}")),
            grounding_confidence: 0.95,
            missing_evidence: Vec::new(),
            conflicting_evidence: Vec::new(),
            partition: if id == 4 {
                EvidencePartitionV2::Current
            } else {
                EvidencePartitionV2::Prior
            },
            causal_parent_ids: Vec::new(),
            committed: Some(true),
            source: "ui_event".into(),
        }
    }

    fn delta(id: usize) -> FrameChangeV2 {
        FrameChangeV2 {
            delta_id: format!("delta-{id}"),
            frame_id: format!("frame-{id}"),
            prior_frame_id: id.checked_sub(1).map(|prior| format!("frame-{prior}")),
            next_frame_id: format!("frame-{id}"),
            diff_kind: Some("content_changed".into()),
            changed_regions: Vec::new(),
            observable_changes: vec!["content appeared".into()],
            no_observable_change: false,
            source_agreement: vec!["accessibility".into()],
            source_conflicts: Vec::new(),
            causal_event_ids: vec![format!("event-{id}")],
            summary_hash: Some(format!("delta-hash-{id}")),
            added_text_hashes: None,
            removed_text_hashes: None,
        }
    }

    fn packet(private_current: bool) -> ObservationPacketV2 {
        let mut frames = (1..=4)
            .map(|id| keyframe(id, private_current && id == 4))
            .collect::<Vec<_>>();
        let current = frames.pop().expect("current frame");
        frames.push(current.clone());
        ObservationPacketV2 {
            schema: "smalltalk.observation_packet.v2".into(),
            packet_id: "packet-probe".into(),
            observed_at_ms: 4_000,
            session_id: Some("session-probe".into()),
            evidence_watermark: "watermark-probe".into(),
            active_surface: surface(4),
            current_frame: current,
            semantic_keyframes: frames,
            canonical_elements: (1..=4).map(element).collect(),
            focused_element_ids: vec!["element-4".into()],
            editable_element_ids: vec!["element-4".into()],
            selected_element_ids: Vec::new(),
            causal_events: (1..=4).map(event).collect(),
            frame_changes: (1..=4).map(delta).collect(),
            capture_trigger_ids: vec!["trigger-manual".into()],
            transition_ids: Vec::new(),
            return_anchor_facts: Vec::new(),
            previous_valid_snapshot_id: None,
            evidence_quality: "strong".into(),
            missing_source_notes: Vec::new(),
            conflicting_observations: Vec::new(),
            partitions: BTreeMap::new(),
            size: PacketSizeAccountingV2 {
                frame_count: 4,
                keyframe_count: 4,
                canonical_element_count: 4,
                causal_event_count: 4,
                serialized_bytes: 1_000,
                estimated_tokens: 250,
                truncated: false,
                frame_accounting: vec![FrameCapacityAccountingV2 {
                    frame_id: "frame-4".into(),
                    partition: EvidencePartitionV2::Current,
                    age_rank: 0,
                    retained_elements: 1,
                    dropped_elements: 0,
                    retained_events: 1,
                    dropped_events: 0,
                    retained_by_source: BTreeMap::new(),
                    dropped_by_source: BTreeMap::new(),
                    retained_by_role: BTreeMap::new(),
                    dropped_by_role: BTreeMap::new(),
                }],
            },
        }
    }

    fn output(primary: Option<&str>) -> ProbeModelOutput {
        let values = BTreeMap::from([
            ("primary_task".into(), vec!["B2_IMAGE_AFTER".into()]),
            ("current_step".into(), vec!["B2_USER_ACTION_1".into()]),
            ("last_progress".into(), vec!["B2_DELTA_1".into()]),
            ("unfinished_state".into(), vec!["B2_OBSERVATION_1".into()]),
        ]);
        ProbeModelOutput {
            primary_task: primary.map(str::to_string),
            current_step: Some("Add support-slot validation".into()),
            last_progress: Some("The probe request was compiled".into()),
            unfinished_state: Some("The real proof corpus remains to be run".into()),
            support_slots_by_field: values,
            missing_evidence: Vec::new(),
            confidence_by_field: SEMANTIC_FIELDS
                .iter()
                .map(|field| ((*field).to_string(), 0.9))
                .collect(),
            status: ProbeResolutionStatus::Resolved,
        }
    }

    #[test]
    fn probe_selects_two_chronological_boundaries_and_stays_under_size_caps() {
        let request = build_probe_request(&packet(false), "model-a").expect("build request");
        assert_eq!(request.audit.boundary_count, 2);
        assert_eq!(request.audit.image_count, 4);
        assert!(request_size_allowed(
            request.audit.structured_bytes,
            request.audit.estimated_text_tokens
        ));
        assert_eq!(
            request.audit.supplied_image_slots,
            vec![
                "B1_IMAGE_BEFORE",
                "B1_IMAGE_AFTER",
                "B2_IMAGE_BEFORE",
                "B2_IMAGE_AFTER"
            ]
        );
        let transported_labels = request
            .body
            .pointer("/input/1/content")
            .and_then(Value::as_array)
            .expect("user content")
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .filter_map(|text| text.strip_prefix("support_slot="))
            .filter_map(|text| text.split_whitespace().next())
            .collect::<Vec<_>>();
        assert_eq!(
            transported_labels,
            request
                .audit
                .supplied_image_slots
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn request_size_limit_accepts_boundary_and_rejects_overflow() {
        assert!(request_size_allowed(
            MAX_TEXT_BYTES,
            MAX_ESTIMATED_TEXT_TOKENS
        ));
        assert!(!request_size_allowed(
            MAX_TEXT_BYTES + 1,
            MAX_ESTIMATED_TEXT_TOKENS
        ));
        assert!(!request_size_allowed(
            MAX_TEXT_BYTES,
            MAX_ESTIMATED_TEXT_TOKENS + 1
        ));
    }

    #[test]
    fn private_current_boundary_is_blocked_before_transport() {
        let error = build_probe_request(&packet(true), "model-a").unwrap_err();
        assert_eq!(error.0, ProbeDiagnosticStatus::PrivacyBlocked);
    }

    #[test]
    fn foreign_slot_nulls_only_that_field_without_semantic_replacement() {
        let packet = packet(false);
        let request = build_probe_request(&packet, "model-a").expect("build request");
        let mut output = output(Some("Implement the PFTU semantic probe"));
        output
            .support_slots_by_field
            .insert("current_step".into(), vec!["FOREIGN_SLOT".into()]);
        let (admitted, issues) = admit_output(&packet, &request, output);
        assert_eq!(
            admitted.primary_task.as_deref(),
            Some("Implement the PFTU semantic probe")
        );
        assert_eq!(admitted.current_step, None);
        assert!(issues
            .iter()
            .any(|issue| issue == "current_step:foreign_or_missing_slot"));
    }

    #[test]
    fn valid_short_slots_round_trip_without_changing_model_semantics() {
        let packet = packet(false);
        let request = build_probe_request(&packet, "model-a").expect("build request");
        let generated = output(Some("Implement the PFTU semantic probe"));
        let (admitted, issues) = admit_output(&packet, &request, generated.clone());
        assert!(issues.is_empty(), "{issues:?}");
        assert_eq!(admitted, generated);
        for field in SEMANTIC_FIELDS {
            assert!(admitted
                .support_slots_by_field
                .get(field)
                .is_some_and(|slots| !slots.is_empty()));
        }
    }

    #[test]
    fn request_schema_and_validator_share_the_same_semantic_support_policy() {
        let packet = packet(false);
        let request = build_probe_request(&packet, "model-a").expect("build request");
        let schema_slots = request
            .body
            .pointer("/text/format/schema/properties/support_slots_by_field/properties/primary_task/items/enum")
            .and_then(Value::as_array)
            .expect("support slot enum")
            .iter()
            .filter_map(Value::as_str)
            .collect::<BTreeSet<_>>();
        let policy_slots = request
            .slots
            .values()
            .filter(|slot| semantic_support_allowed("primary_task", slot.category))
            .map(|slot| slot.slot.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(schema_slots, policy_slots);
        assert!(schema_slots.contains("B1_IMAGE_BEFORE"));
        assert!(!schema_slots.iter().any(|slot| slot.contains("SURFACE")));

        let mut generated = output(Some("Implement the PFTU semantic probe"));
        generated
            .support_slots_by_field
            .insert("primary_task".into(), vec!["B1_IMAGE_BEFORE".into()]);
        let (admitted, issues) = admit_output(&packet, &request, generated);
        assert!(issues.is_empty(), "{issues:?}");
        assert_eq!(
            admitted.primary_task.as_deref(),
            Some("Implement the PFTU semantic probe")
        );
    }

    #[test]
    fn cited_slots_are_preserved_before_field_level_admission() {
        let packet = packet(false);
        let request = build_probe_request(&packet, "model-a").expect("build request");
        let mut generated = output(Some("Implement the PFTU semantic probe"));
        generated
            .support_slots_by_field
            .insert("current_step".into(), vec!["B1_SURFACE_1".into()]);
        let response = json!({"output_text": serde_json::to_string(&generated).unwrap()});
        let (admitted, _, issues, cited_before_admission) =
            parse_probe_response(&packet, &request, &response).expect("parse response");
        assert_eq!(admitted.current_step, None);
        assert_eq!(
            cited_before_admission.get("current_step"),
            Some(&vec!["B1_SURFACE_1".into()])
        );
        assert!(issues
            .iter()
            .any(|issue| issue == "current_step:slot_category_not_allowed_for_field"));
    }

    #[test]
    fn stale_slot_hash_is_rejected_and_field_is_nulled() {
        let mut packet = packet(false);
        let request = build_probe_request(&packet, "model-a").expect("build request");
        packet.canonical_elements[3].visual_description = Some("changed after request".into());
        let (admitted, issues) = admit_output(
            &packet,
            &request,
            output(Some("Implement the PFTU semantic probe")),
        );
        assert_eq!(admitted.unfinished_state, None);
        assert!(issues
            .iter()
            .any(|issue| issue == "unfinished_state:stale_or_ineligible_slot"));
    }

    #[test]
    fn privacy_blocked_slot_is_rejected_and_only_its_field_is_nulled() {
        let packet = packet(false);
        let mut request = build_probe_request(&packet, "model-a").expect("build request");
        request
            .slots
            .get_mut("B2_OBSERVATION_1")
            .expect("owned observation slot")
            .privacy_eligible = false;
        let (admitted, issues) = admit_output(
            &packet,
            &request,
            output(Some("Implement the PFTU semantic probe")),
        );
        assert_eq!(admitted.unfinished_state, None);
        assert_eq!(
            admitted.primary_task.as_deref(),
            Some("Implement the PFTU semantic probe")
        );
        assert!(issues
            .iter()
            .any(|issue| issue == "unfinished_state:stale_or_ineligible_slot"));
    }

    #[test]
    fn generic_primary_task_is_rejected_without_local_repair() {
        let packet = packet(false);
        let request = build_probe_request(&packet, "model-a").expect("build request");
        let (admitted, issues) = admit_output(&packet, &request, output(Some("Editing code")));
        assert_eq!(admitted.primary_task, None);
        assert!(issues
            .iter()
            .any(|issue| issue == "primary_task:forbidden_generic_primary_task"));
    }

    #[test]
    fn parsing_distinguishes_resolved_partial_unresolved_refusal_empty_and_malformed_output() {
        let packet = packet(false);
        let request = build_probe_request(&packet, "model-a").expect("build request");
        let resolved = json!({"output_text":serde_json::to_string(&output(Some("Implement the PFTU semantic probe"))).unwrap()});
        assert_eq!(
            parse_probe_response(&packet, &request, &resolved)
                .expect("resolved response")
                .0
                .status,
            ProbeResolutionStatus::Resolved
        );

        let mut partial_output = output(Some("Implement the PFTU semantic probe"));
        partial_output.current_step = None;
        partial_output
            .support_slots_by_field
            .insert("current_step".into(), Vec::new());
        partial_output
            .confidence_by_field
            .insert("current_step".into(), 0.0);
        partial_output.status = ProbeResolutionStatus::PartlyResolved;
        let partial = json!({"output_text":serde_json::to_string(&partial_output).unwrap()});
        assert_eq!(
            parse_probe_response(&packet, &request, &partial)
                .expect("partial response")
                .0
                .status,
            ProbeResolutionStatus::PartlyResolved
        );

        let mut unresolved_output = output(None);
        unresolved_output.current_step = None;
        unresolved_output.last_progress = None;
        unresolved_output.unfinished_state = None;
        for field in SEMANTIC_FIELDS {
            unresolved_output
                .support_slots_by_field
                .insert(field.into(), Vec::new());
            unresolved_output
                .confidence_by_field
                .insert(field.into(), 0.0);
        }
        unresolved_output.status = ProbeResolutionStatus::Unresolved;
        let unresolved = json!({"output_text":serde_json::to_string(&unresolved_output).unwrap()});
        assert_eq!(
            parse_probe_response(&packet, &request, &unresolved)
                .expect("unresolved response")
                .0
                .status,
            ProbeResolutionStatus::Unresolved
        );

        let refusal = json!({"output":[{"content":[{"type":"refusal"}]}]});
        assert_eq!(
            parse_probe_response(&packet, &request, &refusal)
                .unwrap_err()
                .0,
            ProbeDiagnosticStatus::ProviderNoUsableOutput
        );
        assert_eq!(
            parse_probe_response(&packet, &request, &json!({}))
                .unwrap_err()
                .0,
            ProbeDiagnosticStatus::ProviderNoUsableOutput
        );
        assert_eq!(
            parse_probe_response(&packet, &request, &json!({"output_text":"not json"}))
                .unwrap_err()
                .0,
            ProbeDiagnosticStatus::StructuredParseFailure
        );
    }

    #[test]
    fn provider_failures_keep_timeout_separate_from_unavailable_transport() {
        assert_eq!(
            classify_transport_failure("request timed out after 90 seconds").0,
            ProbeDiagnosticStatus::Timeout
        );
        assert_eq!(
            classify_transport_failure("HTTP 401").0,
            ProbeDiagnosticStatus::ProviderRejected
        );
        assert_eq!(
            classify_transport_failure("model returned HTTP 404").0,
            ProbeDiagnosticStatus::ProviderUnavailable
        );
    }

    #[test]
    fn missing_credentials_is_a_typed_provider_failure_with_no_output() {
        let (attempt, _) = run_probe(&packet(false), "model-a", None);
        assert_eq!(
            attempt.diagnostic_status,
            ProbeDiagnosticStatus::ProviderUnavailable
        );
        assert!(!attempt.parsed_response);
        assert_eq!(attempt.admitted_output, None);
        assert_eq!(
            attempt.failure_reason.as_deref(),
            Some("credentials_missing")
        );
    }

    #[test]
    fn stale_expected_meaning_cannot_be_consumed_by_later_unrelated_work() {
        let now_ms = 2_000_000;
        assert!(validate_expected_timing(now_ms - MAX_ARMED_CASE_AGE_MS, now_ms).is_ok());
        assert_eq!(
            validate_expected_timing(now_ms - MAX_ARMED_CASE_AGE_MS - 1, now_ms).unwrap_err(),
            "probe_expected_meaning_is_stale"
        );
        assert_eq!(
            validate_expected_timing(now_ms, now_ms).unwrap_err(),
            "probe_expected_meaning_was_not_recorded_before_output"
        );
    }

    #[test]
    fn passive_page_prompt_does_not_turn_visible_content_into_user_intent() {
        let instruction = system_instruction();
        assert!(instruction.contains("Never rewrite the purpose of visible page content"));
        assert!(instruction.contains("primary_task must be null"));
    }

    #[test]
    #[ignore = "makes real provider calls with a synthetic repository image"]
    fn live_probe_transport_smoke_uses_configured_cost_model() {
        let (api_key, model) = configured_model().expect("load existing secure provider config");
        let api_key = api_key.expect("OPENAI_API_KEY must be configured");
        let image_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("icons/128x128.png");
        assert!(image_path.is_file(), "repository icon is missing");
        let mut packet = packet(false);
        for frame in &mut packet.semantic_keyframes {
            frame.ephemeral_local_image_path = Some(image_path.to_string_lossy().into_owned());
            frame.local_image_handle_hash = Some(fingerprint(&std::fs::read(&image_path).unwrap()));
        }
        packet.current_frame.ephemeral_local_image_path =
            Some(image_path.to_string_lossy().into_owned());
        packet.current_frame.local_image_handle_hash =
            Some(fingerprint(&std::fs::read(&image_path).unwrap()));
        let (attempt, _) = run_probe(&packet, &model, Some(&api_key));
        eprintln!(
                "model={} diagnostic={:?} parsed={} request_bytes={:?} tokens={:?} response_id_present={} latency_ms={} input_tokens={:?} output_tokens={:?} cost_usd={:?} output_bytes={:?}",
                model,
                attempt.diagnostic_status,
                attempt.parsed_response,
                attempt
                    .request_audit
                    .as_ref()
                    .map(|audit| audit.structured_bytes),
                attempt
                    .request_audit
                    .as_ref()
                    .map(|audit| audit.estimated_text_tokens),
                attempt.response_id.is_some(),
                attempt.latency_ms,
                attempt.usage.input_tokens,
                attempt.usage.output_tokens,
                attempt.estimated_cost_usd,
                attempt.output_bytes
        );
        assert!(
            attempt.parsed_response,
            "{model} failed: {:?}",
            attempt.failure_reason
        );
    }

    fn proof_attempt(model: &str) -> ProofAttempt {
        ProofAttempt {
            model: model.into(),
            real_provider_round_trip: true,
            diagnostic_status: ProbeDiagnosticStatus::Success,
            parsed_response: true,
            provider_request_id_present: true,
            response_id_present: true,
            response_recorded_at_ms: 2_000,
            request_bytes: 8_000,
            request_estimated_tokens: 2_000,
            image_count: 4,
            output_bytes: 800,
            input_tokens: Some(3_000),
            output_tokens: Some(200),
            estimated_cost_usd: Some(0.01),
            latency_ms: 1_000,
            output_status: ProbeResolutionStatus::Resolved,
            output_by_field: SEMANTIC_FIELDS
                .iter()
                .map(|field| ((*field).into(), Some(format!("value for {field}"))))
                .collect(),
            confidence_by_field: SEMANTIC_FIELDS
                .iter()
                .map(|field| ((*field).into(), 0.9))
                .collect(),
            cited_support_slots_by_field: SEMANTIC_FIELDS
                .iter()
                .map(|field| ((*field).into(), vec!["B1_IMAGE_AFTER".into()]))
                .collect(),
            support_admitted_by_field: SEMANTIC_FIELDS
                .iter()
                .map(|field| ((*field).into(), true))
                .collect(),
            unsupported_fields_null_or_rejected: true,
            local_semantic_fallback_used: false,
            judgments_by_field: SEMANTIC_FIELDS
                .iter()
                .map(|field| ((*field).into(), FieldJudgment::Correct))
                .collect(),
            corrections_by_field: SEMANTIC_FIELDS
                .iter()
                .map(|field| ((*field).into(), "none".into()))
                .collect(),
            concrete_without_app_or_generic_verb: true,
        }
    }

    fn passing_corpus() -> ProofCorpus {
        ProofCorpus {
            schema: PROBE_CORPUS_SCHEMA.into(),
            frozen_before_holdout: true,
            current_model: "model-a".into(),
            chosen_model: "model-a".into(),
            old_request: OldRequestMeasurement {
                source_session: "session-038".into(),
                structured_bytes: 100_000,
                estimated_tokens: 25_000,
                image_count: 4,
                max_output_tokens: 6_000,
            },
            cases: REQUIRED_CASE_KINDS
                .iter()
                .enumerate()
                .map(|(index, kind)| ProofCase {
                    case_id: format!("pftu-case-{:02}", index + 1),
                    case_kind: (*kind).into(),
                    held_back: index >= 8,
                    case_timestamp_ms: 1_500,
                    session_id: format!("session-{index}"),
                    decision_id: format!("decision-{index}"),
                    expected_recorded_at_ms: 1_000,
                    expected_by_field: SEMANTIC_FIELDS
                        .iter()
                        .map(|field| ((*field).into(), Some(format!("expected {field}"))))
                        .collect(),
                    recoverable_by_field: SEMANTIC_FIELDS
                        .iter()
                        .map(|field| ((*field).into(), true))
                        .collect(),
                    human_reviewed: true,
                    reviewer_id: "reviewer-1".into(),
                    attempts: vec![proof_attempt("model-a")],
                })
                .collect(),
        }
    }

    #[test]
    fn proof_gate_passes_only_complete_denominator_safe_real_corpus() {
        let report = evaluate_proof_corpus(&passing_corpus());
        assert!(report.passed, "{:?}", report.violations);

        let mut empty_denominator = passing_corpus();
        for case in &mut empty_denominator.cases {
            for recoverable in case.recoverable_by_field.values_mut() {
                *recoverable = false;
            }
        }
        let report = evaluate_proof_corpus(&empty_denominator);
        assert!(!report.passed);
        assert!(report
            .violations
            .iter()
            .any(|violation| violation.starts_with("recoverable_field_quality_below")));
    }

    #[test]
    fn proof_gate_rejects_confident_wrong_primary_task() {
        let mut corpus = passing_corpus();
        corpus.cases[0].attempts[0]
            .judgments_by_field
            .insert("primary_task".into(), FieldJudgment::Wrong);
        let report = evaluate_proof_corpus(&corpus);
        assert!(!report.passed);
        assert_eq!(report.metrics.confident_wrong_primary_task_count, 1);
    }
}
