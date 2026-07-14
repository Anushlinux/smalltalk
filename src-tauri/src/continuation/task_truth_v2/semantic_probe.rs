use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use super::model::{self, ProviderUsageV1};
use super::observation_packet::{
    is_private_status, AuthorshipStatusV2, EvidencePartitionV2, ObservationPacketV2, RegionRoleV2,
};

pub(crate) const PROBE_RESPONSE_SCHEMA: &str = "smalltalk.pftu_01.semantic_probe_response.v2";
pub(crate) const PROBE_REQUEST_SCHEMA: &str = "smalltalk.pftu_01.semantic_probe_request.v3";
pub(crate) const PROBE_CORPUS_SCHEMA: &str = "smalltalk.pftu_01.proof_corpus.v1";
const DEFAULT_LUNA_MODEL: &str = "gpt-5.6-luna";
const MAX_BOUNDARIES: usize = 2;
const MAX_IMAGES: usize = 4;
const MAX_TEXT_BYTES: usize = 24 * 1024;
const MAX_ESTIMATED_TEXT_TOKENS: usize = 6_144;
// The Responses API counts model reasoning and the final structured JSON
// against the same output budget. The response contract can contain four
// semantic fields plus one role object for each of the four supplied images.
// The former 1,200-token limit was exhausted before that JSON was complete.
const MAX_OUTPUT_TOKENS: usize = 6_000;
const MAX_OBSERVATIONS_PER_BOUNDARY: usize = 6;
const MAX_ACTIONS_PER_BOUNDARY: usize = 4;
const MAX_DELTAS_PER_BOUNDARY: usize = 3;
const MAX_SEMANTIC_FIELD_CHARS: usize = 320;
const MAX_MISSING_EVIDENCE_CHARS: usize = 240;
const MAX_ARMED_CASE_AGE_MS: i64 = 15 * 60 * 1_000;
const MANUAL_PROVIDER_RETRIES: u32 = 0;
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
pub(crate) enum ProbeSurfaceRole {
    PrimaryWork,
    SupportingWork,
    DetourOrUnrelated,
    Unclear,
}

impl ProbeSurfaceRole {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::PrimaryWork => "primary_work",
            Self::SupportingWork => "supporting_work",
            Self::DetourOrUnrelated => "detour_or_unrelated",
            Self::Unclear => "unclear",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProbeVisitRole {
    pub(crate) role: ProbeSurfaceRole,
    pub(crate) confidence: f64,
    pub(crate) support_slots: Vec<String>,
    pub(crate) relationship_to_primary_task: String,
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
    ContextImage,
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
    #[serde(default)]
    pub(crate) visit_roles: BTreeMap<String, ProbeVisitRole>,
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
    #[serde(default)]
    pub(crate) max_output_tokens: usize,
    pub(crate) output_contract_field_count: usize,
    pub(crate) supplied_image_slots: Vec<String>,
    pub(crate) missing_evidence: Vec<String>,
    #[serde(default)]
    pub(crate) final_frame_id: String,
    #[serde(default)]
    pub(crate) cutoff_observed_at_ms: i64,
    #[serde(default)]
    pub(crate) earliest_included_at_ms: i64,
    #[serde(default)]
    pub(crate) boundary_selection_reasons: Vec<String>,
    #[serde(default)]
    pub(crate) raw_candidate_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub(crate) admitted_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub(crate) deduplication_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub(crate) excluded_late_records_by_kind: BTreeMap<String, usize>,
    #[serde(default)]
    pub(crate) excluded_nonsemantic_records_by_kind: BTreeMap<String, usize>,
    #[serde(default)]
    pub(crate) surface_timeline: Vec<ProbeSurfaceVisitAudit>,
    #[serde(default)]
    pub(crate) image_selection_reasons: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) omitted_surface_visit_count: usize,
    #[serde(default)]
    pub(crate) image_exclusions_by_reason: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ProbeSurfaceVisitAudit {
    #[serde(default)]
    pub(crate) visit_id: String,
    pub(crate) sequence_index: usize,
    pub(crate) app_label: String,
    pub(crate) site_hostname: Option<String>,
    pub(crate) first_observed_at_ms: i64,
    pub(crate) last_observed_at_ms: i64,
    pub(crate) is_current: bool,
    pub(crate) revisited: bool,
    pub(crate) private: bool,
    pub(crate) image_slot: Option<String>,
    #[serde(default)]
    pub(crate) image_omission_reason: Option<String>,
    #[serde(default)]
    pub(crate) representative_frame_reasons: Vec<String>,
    pub(crate) evidence_refs: Vec<String>,
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
    #[serde(default)]
    pub(crate) provider_post_count: usize,
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
    chronology_role: &'a str,
    selection_reason: &'a str,
    surface_relation: &'a str,
    observed_transition: String,
    slots: Vec<RequestSlot<'a>>,
}

#[derive(Debug, Clone, Serialize)]
struct RequestSurfaceVisit<'a> {
    visit_id: String,
    sequence_index: usize,
    app_label: &'a str,
    site_hostname: Option<&'a str>,
    first_observed_at_ms: i64,
    last_observed_at_ms: i64,
    is_current: bool,
    revisited: bool,
    image_slot: Option<&'a str>,
}

#[derive(Debug, Clone)]
struct SelectedBoundary {
    selection_reason: String,
    surface_relation: String,
    frames: Vec<super::observation_packet::KeyframeReferenceV2>,
}

#[derive(Debug, Clone, Default)]
struct SlotBuildAudit {
    raw_candidate_counts: BTreeMap<String, usize>,
    admitted_counts: BTreeMap<String, usize>,
    deduplication_counts: BTreeMap<String, usize>,
    excluded_late_records_by_kind: BTreeMap<String, usize>,
    excluded_nonsemantic_records_by_kind: BTreeMap<String, usize>,
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

fn physical_frame_fingerprint(frame: &super::observation_packet::KeyframeReferenceV2) -> String {
    fingerprint(&json!({
        "frame_id": frame.frame_id,
        "observed_at_ms": frame.observed_at_ms,
        "surface_identity": frame.surface_identity,
        "surface_ownership_confidence": frame.surface_ownership_confidence,
        "privacy_status": frame.privacy_status,
        "model_eligible": frame.model_eligible,
        "image_source_kind": frame.image_source_kind,
        "image_scope": frame.image_scope,
        "image_width": frame.image_width,
        "image_height": frame.image_height,
        "image_rejection_reason": frame.image_rejection_reason,
        "crop_pixels": frame.crop_pixels,
        "local_image_handle_hash": frame.local_image_handle_hash,
    }))
}

fn slot_name(boundary: usize, suffix: &str) -> String {
    format!("B{boundary}_{suffix}")
}

fn insert_slot(slots: &mut BTreeMap<String, SupportSlot>, slot: SupportSlot) {
    slots.insert(slot.slot.clone(), slot);
}

fn find_packet_frame<'a>(
    packet: &'a ObservationPacketV2,
    frame_id: &str,
) -> Option<&'a super::observation_packet::KeyframeReferenceV2> {
    packet
        .semantic_keyframes
        .iter()
        .chain(std::iter::once(&packet.current_frame))
        .chain(
            packet
                .surface_timeline
                .iter()
                .filter_map(|visit| visit.representative_frame.as_ref()),
        )
        .find(|frame| frame.frame_id == frame_id)
}

fn packet_frame_variants<'a>(
    packet: &'a ObservationPacketV2,
    frame_id: &'a str,
) -> impl Iterator<Item = &'a super::observation_packet::KeyframeReferenceV2> + 'a {
    packet
        .semantic_keyframes
        .iter()
        .chain(std::iter::once(&packet.current_frame))
        .chain(
            packet
                .surface_timeline
                .iter()
                .filter_map(|visit| visit.representative_frame.as_ref()),
        )
        .filter(move |frame| frame.frame_id == frame_id)
}

fn frame_time(packet: &ObservationPacketV2, frame_id: &str) -> Option<i64> {
    find_packet_frame(packet, frame_id).map(|frame| frame.observed_at_ms)
}

fn same_surface(
    left: &super::observation_packet::KeyframeReferenceV2,
    right: &super::observation_packet::KeyframeReferenceV2,
) -> bool {
    let left_identity = &left.surface_identity;
    let right_identity = &right.surface_identity;
    let same_app = left_identity.app_bundle_id.is_some()
        && left_identity.app_bundle_id == right_identity.app_bundle_id;
    let same_owned_object = [
        left_identity
            .document_path_hash
            .as_ref()
            .zip(right_identity.document_path_hash.as_ref()),
        left_identity
            .browser_url_hash
            .as_ref()
            .zip(right_identity.browser_url_hash.as_ref()),
        left_identity
            .window_title_hash
            .as_ref()
            .zip(right_identity.window_title_hash.as_ref()),
    ]
    .into_iter()
    .flatten()
    .any(|(left, right)| left == right)
        || (left_identity.window_id.is_some()
            && left_identity.window_id == right_identity.window_id);
    same_app && same_owned_object
}

fn same_application(
    left: &super::observation_packet::KeyframeReferenceV2,
    right: &super::observation_packet::KeyframeReferenceV2,
) -> bool {
    left.surface_identity.app_bundle_id.is_some()
        && left.surface_identity.app_bundle_id == right.surface_identity.app_bundle_id
}

fn continued_activity_on_surface(
    packet: &ObservationPacketV2,
    frame: &super::observation_packet::KeyframeReferenceV2,
    cutoff: i64,
) -> bool {
    packet.causal_events.iter().any(|event| {
        event.observed_at_ms >= frame.observed_at_ms
            && event.observed_at_ms <= cutoff
            && event.app_bundle_id.is_some()
            && event.app_bundle_id == frame.surface_identity.app_bundle_id
            && event_is_meaningful(packet, event, cutoff)
    })
}

fn observed_boundary_transition(boundary: &SelectedBoundary) -> String {
    let Some(first) = boundary.frames.first() else {
        return "No readable screen was available for this boundary.".into();
    };
    let Some(last) = boundary.frames.last() else {
        return "No readable screen was available for this boundary.".into();
    };
    if first.frame_id == last.frame_id {
        return "One screen represents this point in the chronology.".into();
    }
    if same_surface(first, last) {
        "The before and after screens remain on the same app surface while the visible state changes."
            .into()
    } else if same_application(first, last) {
        "The user moved to a different page, document, or window within the same application."
            .into()
    } else {
        "The foreground application changed between the before and after screens.".into()
    }
}

fn event_has_material_result(
    packet: &ObservationPacketV2,
    event: &super::observation_packet::CausalEventV2,
    cutoff: i64,
) -> bool {
    packet.frame_changes.iter().any(|delta| {
        let belongs_to_event = if let Some(reference) = event.semantic_delta_reference.as_deref() {
            reference == delta.delta_id
        } else if delta
            .causal_event_ids
            .iter()
            .any(|id| id == &event.event_id)
        {
            true
        } else {
            event.target_frame_id.as_deref() == Some(delta.next_frame_id.as_str())
        };
        !delta.no_observable_change
            && frame_time(packet, &delta.next_frame_id).is_some_and(|time| time <= cutoff)
            && belongs_to_event
    })
}

fn event_is_meaningful(
    packet: &ObservationPacketV2,
    event: &super::observation_packet::CausalEventV2,
    cutoff: i64,
) -> bool {
    if event.observed_at_ms > cutoff || event.grounding_confidence < 0.5 {
        return false;
    }
    if let Some(target_frame_id) = event.target_frame_id.as_deref() {
        let Some(target_time) = frame_time(packet, target_frame_id) else {
            // An unrepresented target cannot be proven to precede the manual
            // cutoff. Fail closed instead of treating an unknown edge as safe.
            return false;
        };
        if target_time > cutoff {
            return false;
        }
    }
    let kind = event.event_kind.trim().to_ascii_lowercase();
    let source = event.source.trim().to_ascii_lowercase();
    if [
        "notification",
        "accessibility",
        "focus",
        "window_metadata",
        "capture",
        "bookkeeping",
    ]
    .iter()
    .any(|passive| kind.contains(passive))
        || (source.contains("accessibility")
            && ["focus", "window", "notification", "changed"]
                .iter()
                .any(|passive| kind.contains(passive)))
    {
        return false;
    }
    if kind.contains("scroll") {
        return event_has_material_result(packet, event, cutoff);
    }
    if kind.contains("app_switch") {
        let target_time = event
            .target_frame_id
            .as_deref()
            .and_then(|frame_id| frame_time(packet, frame_id));
        return target_time.is_some_and(|time| time <= cutoff)
            && packet.causal_events.iter().any(|later| {
                later.observed_at_ms > event.observed_at_ms
                    && later.observed_at_ms <= cutoff
                    && later.app_bundle_id == event.app_bundle_id
                    && later.event_id != event.event_id
                    && !later.event_kind.to_ascii_lowercase().contains("focus")
            });
    }
    event.committed == Some(true)
        || [
            "submit",
            "send",
            "command",
            "terminal",
            "typing_commit",
            "committed_typing",
        ]
        .iter()
        .any(|signal| kind.contains(signal))
        || (["click", "navigation"]
            .iter()
            .any(|signal| kind.contains(signal))
            && event_has_material_result(packet, event, cutoff))
}

fn delta_has_grounded_cause(
    packet: &ObservationPacketV2,
    delta: &super::observation_packet::FrameChangeV2,
    cutoff: i64,
) -> bool {
    packet.causal_events.iter().any(|event| {
        (delta
            .causal_event_ids
            .iter()
            .any(|id| id == &event.event_id)
            || event.semantic_delta_reference.as_deref() == Some(delta.delta_id.as_str())
            || event.target_frame_id.as_deref() == Some(delta.next_frame_id.as_str()))
            && event_is_meaningful(packet, event, cutoff)
            && event_has_material_result(packet, event, cutoff)
    })
}

fn selected_boundaries(
    packet: &ObservationPacketV2,
) -> Result<Vec<SelectedBoundary>, (ProbeDiagnosticStatus, String)> {
    let current = &packet.current_frame;
    if current.frame_id.trim().is_empty() {
        return Err((
            ProbeDiagnosticStatus::RequestNotBuilt,
            "current_frame_missing".into(),
        ));
    }
    if current.observed_at_ms != packet.observed_at_ms {
        return Err((
            ProbeDiagnosticStatus::RequestNotBuilt,
            "current_frame_stale".into(),
        ));
    }
    if is_private_status(Some(&current.privacy_status)) {
        return Err((
            ProbeDiagnosticStatus::PrivacyBlocked,
            "current_frame_privacy_blocked".into(),
        ));
    }
    if !current.model_eligible {
        return Err((
            ProbeDiagnosticStatus::RequestNotBuilt,
            "current_frame_model_ineligible".into(),
        ));
    }
    let cutoff = current.observed_at_ms;
    let mut eligible_frames = packet
        .semantic_keyframes
        .iter()
        .chain(std::iter::once(current))
        .filter(|frame| {
            frame.observed_at_ms <= cutoff
                && frame.partition != EvidencePartitionV2::Background
                && frame.model_eligible
                && !is_private_status(Some(&frame.privacy_status))
        })
        .cloned()
        .collect::<Vec<_>>();
    eligible_frames.sort_by_key(|frame| (frame.observed_at_ms, frame.frame_id.clone()));
    eligible_frames.dedup_by(|left, right| left.frame_id == right.frame_id);

    let frame_by_id = |frame_id: &str| {
        eligible_frames
            .iter()
            .find(|frame| frame.frame_id == frame_id)
            .cloned()
    };
    let direct_prior = packet
        .frame_changes
        .iter()
        .filter(|delta| {
            !delta.no_observable_change
                && delta.next_frame_id == current.frame_id
                && frame_time(packet, &delta.next_frame_id).is_some_and(|time| time <= cutoff)
                && delta_has_grounded_cause(packet, delta, cutoff)
        })
        .filter_map(|delta| delta.prior_frame_id.as_deref().and_then(frame_by_id))
        .max_by_key(|frame| (frame.observed_at_ms, frame.frame_id.clone()))
        .or_else(|| {
            packet
                .causal_events
                .iter()
                .filter(|event| {
                    event.target_frame_id.as_deref() == Some(current.frame_id.as_str())
                        && event_is_meaningful(packet, event, cutoff)
                })
                .filter_map(|event| frame_by_id(&event.source_frame_id))
                .max_by_key(|frame| (frame.observed_at_ms, frame.frame_id.clone()))
        });
    let mut current_frames = direct_prior.into_iter().collect::<Vec<_>>();
    if !current_frames
        .iter()
        .any(|frame| frame.frame_id == current.frame_id)
    {
        current_frames.push(current.clone());
    }
    current_frames.sort_by_key(|frame| (frame.observed_at_ms, frame.frame_id.clone()));
    current_frames.dedup_by(|left, right| left.frame_id == right.frame_id);
    let current_start = current_frames
        .first()
        .map(|frame| frame.observed_at_ms)
        .unwrap_or(cutoff);

    let current_boundary_has_grounded_result = packet.causal_events.iter().any(|event| {
        event.target_frame_id.as_deref() == Some(current.frame_id.as_str())
            && event_is_meaningful(packet, event, cutoff)
            && event_has_material_result(packet, event, cutoff)
    });
    let current_frame_ids = current_frames
        .iter()
        .map(|frame| frame.frame_id.as_str())
        .collect::<BTreeSet<_>>();

    // A local scroll or click can prove that the current screen changed, but
    // it cannot prove that the current screen contains enough task context.
    // Preserve one recent transition into the current surface when the user
    // continued acting there. This keeps a compact view of detours, returns,
    // and supporting surfaces instead of reducing every request to two nearly
    // identical final screenshots.
    let surface_transition_context = packet
        .frame_changes
        .iter()
        .filter(|delta| !delta.no_observable_change)
        .filter_map(|delta| {
            let prior = delta.prior_frame_id.as_deref().and_then(frame_by_id)?;
            let next = frame_by_id(&delta.next_frame_id)?;
            if next.observed_at_ms > current_start
                || !same_surface(&next, current)
                || same_surface(&prior, &next)
                || (current_frame_ids.contains(prior.frame_id.as_str())
                    && current_frame_ids.contains(next.frame_id.as_str()))
            {
                return None;
            }
            let context_is_grounded = delta_has_grounded_cause(packet, delta, cutoff)
                || continued_activity_on_surface(packet, &next, cutoff);
            if !context_is_grounded {
                return None;
            }
            let returned_to_prior_surface = eligible_frames.iter().any(|frame| {
                frame.observed_at_ms < prior.observed_at_ms && same_surface(frame, current)
            });
            Some(SelectedBoundary {
                selection_reason: if returned_to_prior_surface {
                    "return_to_prior_surface"
                } else {
                    "recent_surface_transition_with_activity"
                }
                .into(),
                surface_relation: "transition_into_current_surface".into(),
                frames: vec![prior, next],
            })
        })
        .max_by_key(|boundary| {
            boundary
                .frames
                .last()
                .map(|frame| (frame.observed_at_ms, frame.frame_id.clone()))
        });

    let earlier = surface_transition_context.or_else(|| {
        if current_boundary_has_grounded_result {
            return None;
        }
        packet
            .frame_changes
            .iter()
            .filter(|delta| !delta.no_observable_change)
            .filter_map(|delta| {
                let prior = delta.prior_frame_id.as_deref().and_then(frame_by_id)?;
                let next = frame_by_id(&delta.next_frame_id)?;
                if next.observed_at_ms >= current_start || !same_surface(&next, current) {
                    return None;
                }
                delta_has_grounded_cause(packet, delta, cutoff).then_some(SelectedBoundary {
                    selection_reason: "committed_action_with_result".into(),
                    surface_relation: "same_surface_as_current".into(),
                    frames: vec![prior, next],
                })
            })
            .max_by_key(|boundary| {
                boundary
                    .frames
                    .last()
                    .map(|frame| (frame.observed_at_ms, frame.frame_id.clone()))
            })
    });

    let current_boundary = SelectedBoundary {
        selection_reason: "current_manual_boundary".into(),
        surface_relation: "current_surface".into(),
        frames: current_frames,
    };
    let mut boundaries = earlier.into_iter().collect::<Vec<_>>();
    boundaries.push(current_boundary);
    if boundaries.is_empty() || boundaries.len() > MAX_BOUNDARIES {
        return Err((
            ProbeDiagnosticStatus::RequestNotBuilt,
            "invalid_boundary_count".into(),
        ));
    }
    Ok(boundaries)
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

fn action_slot_summary(
    packet: &ObservationPacketV2,
    event: &super::observation_packet::CausalEventV2,
    cutoff: i64,
) -> String {
    let kind = event.event_kind.trim().to_ascii_lowercase();
    let result = event_has_material_result(packet, event, cutoff);
    let result_sentence = if result {
        " A later captured screen shows an observable result."
    } else {
        " No distinct observable result was captured."
    };
    let action = if kind.contains("scroll") {
        "The user scrolled on this surface.".to_string()
    } else if kind.contains("click") {
        if event.target_element_id.is_some() {
            "The user clicked a captured control or content region.".to_string()
        } else {
            "The user clicked, but the clicked target was not reliably identified.".to_string()
        }
    } else if kind.contains("app_switch") {
        "The foreground application changed, followed by activity on the new surface.".to_string()
    } else if kind.contains("navigation") {
        "The user navigated to a different visible surface.".to_string()
    } else if kind.contains("command") || kind.contains("terminal") {
        "The user committed or ran a command.".to_string()
    } else if kind.contains("typing") || event.committed == Some(true) {
        "The user committed text input.".to_string()
    } else {
        let readable_kind = kind.replace('_', " ").replace('-', " ");
        format!("The user performed a {readable_kind} action.")
    };
    format!("{action}{result_sentence}")
}

fn delta_slot_summary(
    packet: &ObservationPacketV2,
    delta: &super::observation_packet::FrameChangeV2,
) -> String {
    let prior = delta
        .prior_frame_id
        .as_deref()
        .and_then(|frame_id| find_packet_frame(packet, frame_id));
    let next = find_packet_frame(packet, &delta.next_frame_id);
    let visible_content_changed = delta
        .observable_changes
        .iter()
        .any(|change| matches!(change.as_str(), "content_appeared" | "content_disappeared"))
        || !delta.changed_regions.is_empty();

    match (prior, next) {
        (Some(prior), Some(next)) if !same_application(prior, next) => {
            "The foreground application changed between the two captured screens.".into()
        }
        (Some(prior), Some(next)) if !same_surface(prior, next) => {
            "The user moved to a different page, document, or window within the same application."
                .into()
        }
        (Some(_), Some(_)) if visible_content_changed => {
            "The screen remained on the same surface while visible content changed.".into()
        }
        (Some(_), Some(_)) => "The captures remain on the same surface. Activity occurred between them, but the packet does not claim what that activity meant.".into(),
        _ if visible_content_changed => {
            "Visible content changed, but the packet cannot reliably name the surrounding surface transition.".into()
        }
        _ => "A state change was recorded, but its visible meaning is not reliably identified."
            .into(),
    }
}

fn surface_visit_identity(visit: &super::observation_packet::SurfaceVisitV2) -> String {
    format!(
        "{}|{}",
        visit.app_label.trim().to_ascii_lowercase(),
        visit
            .site_hostname
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase()
    )
}

fn selected_context_image_visits(
    packet: &ObservationPacketV2,
    cutoff: i64,
) -> Vec<&super::observation_packet::SurfaceVisitV2> {
    let current_identity = packet
        .surface_timeline
        .iter()
        .find(|visit| visit.is_current)
        .map(surface_visit_identity);
    let mut best_by_surface = BTreeMap::<String, &super::observation_packet::SurfaceVisitV2>::new();
    for visit in &packet.surface_timeline {
        let identity = surface_visit_identity(visit);
        let Some(frame) = visit.representative_frame.as_ref() else {
            continue;
        };
        if visit.is_current
            || visit.private
            || current_identity.as_deref() == Some(identity.as_str())
            || frame.observed_at_ms > cutoff
            || !frame.model_eligible
            || is_private_status(Some(&frame.privacy_status))
        {
            continue;
        }
        let replace = best_by_surface.get(&identity).is_none_or(|existing| {
            (
                visit.engagement_score,
                visit.last_observed_at_ms,
                visit.sequence_index,
            ) > (
                existing.engagement_score,
                existing.last_observed_at_ms,
                existing.sequence_index,
            )
        });
        if replace {
            best_by_surface.insert(identity, visit);
        }
    }
    let mut selected = best_by_surface.into_values().collect::<Vec<_>>();
    selected.sort_by_key(|visit| {
        (
            std::cmp::Reverse(visit.engagement_score),
            std::cmp::Reverse(visit.last_observed_at_ms),
            visit.sequence_index,
        )
    });
    selected.truncate(MAX_IMAGES.saturating_sub(1));
    selected.sort_by_key(|visit| visit.sequence_index);
    selected
}

fn surface_image_omission_reason(
    visit: &super::observation_packet::SurfaceVisitV2,
    selected_image_frame_ids: &BTreeSet<&str>,
) -> Option<&'static str> {
    if visit.private {
        return Some("private");
    }
    let Some(frame) = visit.representative_frame.as_ref() else {
        return Some("missing_crop");
    };
    if selected_image_frame_ids.contains(frame.frame_id.as_str()) {
        return None;
    }
    if frame.model_eligible {
        return Some("budget_omitted");
    }
    let rejection = frame.image_rejection_reason.as_deref().unwrap_or("");
    if rejection.contains("ownership") || rejection.contains("coordinate_mapping") {
        Some("ownership_rejected")
    } else if rejection.contains("crop")
        || rejection.contains("image")
        || rejection.contains("readable")
    {
        Some("missing_crop")
    } else {
        Some("model_ineligible")
    }
}

fn build_slots(
    packet: &ObservationPacketV2,
    boundaries: &[SelectedBoundary],
    cutoff: i64,
) -> (BTreeMap<String, SupportSlot>, SlotBuildAudit) {
    let mut slots = BTreeMap::new();
    let mut audit = SlotBuildAudit::default();
    let raw_keyframe_count = packet
        .semantic_keyframes
        .iter()
        .map(|frame| frame.frame_id.as_str())
        .chain(std::iter::once(packet.current_frame.frame_id.as_str()))
        .chain(
            packet
                .surface_timeline
                .iter()
                .filter_map(|visit| visit.representative_frame.as_ref())
                .map(|frame| frame.frame_id.as_str()),
        )
        .collect::<BTreeSet<_>>()
        .len();
    audit.raw_candidate_counts = BTreeMap::from([
        ("keyframe".into(), raw_keyframe_count),
        (
            "canonical_observation".into(),
            packet.canonical_elements.len(),
        ),
        ("causal_event".into(), packet.causal_events.len()),
        ("semantic_delta".into(), packet.frame_changes.len()),
    ]);
    audit.excluded_late_records_by_kind = BTreeMap::from([
        (
            "keyframe".into(),
            packet
                .semantic_keyframes
                .iter()
                .filter(|frame| frame.observed_at_ms > cutoff)
                .count(),
        ),
        (
            "canonical_observation".into(),
            packet
                .canonical_elements
                .iter()
                .filter(|element| element.changed_at_ms > cutoff)
                .count(),
        ),
        (
            "causal_event".into(),
            packet
                .causal_events
                .iter()
                .filter(|event| {
                    event.observed_at_ms > cutoff
                        || event
                            .target_frame_id
                            .as_deref()
                            .and_then(|frame_id| frame_time(packet, frame_id))
                            .is_some_and(|time| time > cutoff)
                })
                .count(),
        ),
        (
            "semantic_delta".into(),
            packet
                .frame_changes
                .iter()
                .filter(|delta| {
                    frame_time(packet, &delta.next_frame_id).is_some_and(|time| time > cutoff)
                })
                .count(),
        ),
    ]);

    let mut reason_strings = packet
        .canonical_elements
        .iter()
        .filter(|element| element.changed_at_ms <= cutoff)
        .flat_map(|element| {
            element
                .source_conflicts
                .iter()
                .chain(element.rejection_reasons.iter())
        })
        .map(|reason| bounded_text(reason, MAX_MISSING_EVIDENCE_CHARS))
        .collect::<Vec<_>>();
    let raw_reason_count = reason_strings.len();
    reason_strings.sort();
    reason_strings.dedup();
    audit
        .raw_candidate_counts
        .insert("reason_string".into(), raw_reason_count);
    audit.deduplication_counts.insert(
        "reason_string".into(),
        raw_reason_count.saturating_sub(reason_strings.len()),
    );

    let context_image_visits = selected_context_image_visits(packet, cutoff);
    let mut emitted_image_frame_ids = BTreeSet::new();
    let mut emitted_observation_ids = BTreeSet::new();
    let mut emitted_event_ids = BTreeSet::new();
    let mut emitted_delta_ids = BTreeSet::new();
    for visit in &context_image_visits {
        let Some(frame) = visit.representative_frame.as_ref() else {
            continue;
        };
        if !emitted_image_frame_ids.insert(frame.frame_id.clone()) {
            continue;
        }
        insert_slot(
            &mut slots,
            SupportSlot {
                slot: format!("T{}_CONTEXT_IMAGE", visit.sequence_index),
                boundary_index: 0,
                category: SupportCategory::ContextImage,
                source_kind: "keyframe".into(),
                record_id: frame.frame_id.clone(),
                frame_id: Some(frame.frame_id.clone()),
                content_hash: frame.local_image_handle_hash.clone(),
                source_fingerprint: physical_frame_fingerprint(frame),
                observed_at_ms: frame.observed_at_ms,
                privacy_eligible: frame.model_eligible
                    && !is_private_status(Some(&frame.privacy_status)),
                ownership_eligible: true,
                summary: "Representative screen from an earlier observed session surface.".into(),
            },
        );
    }
    let allow_current_before = context_image_visits.len() + 2 <= MAX_IMAGES;
    let use_boundary_image_fallback = context_image_visits.is_empty();
    for (assignment_index, boundary) in boundaries.iter().enumerate() {
        let boundary_index = assignment_index + 1;
        let boundary_frames = &boundary.frames;
        let boundary_frame_ids = boundary_frames
            .iter()
            .map(|frame| frame.frame_id.as_str())
            .collect::<BTreeSet<_>>();
        let boundary_start_ms = boundary_frames
            .first()
            .map(|frame| frame.observed_at_ms)
            .unwrap_or(cutoff);
        let boundary_end_ms = boundary_frames
            .last()
            .map(|frame| frame.observed_at_ms)
            .unwrap_or(cutoff);

        let is_current_boundary = assignment_index + 1 == boundaries.len();
        for (position, frame) in boundary_frames.iter().enumerate() {
            if !is_current_boundary && !use_boundary_image_fallback {
                continue;
            }
            let is_current_frame = frame.frame_id == packet.current_frame.frame_id;
            if !is_current_frame && !allow_current_before {
                continue;
            }
            if !is_current_frame
                && frame.local_image_handle_hash.is_some()
                && frame.local_image_handle_hash == packet.current_frame.local_image_handle_hash
            {
                continue;
            }
            if !is_current_frame && emitted_image_frame_ids.len() >= MAX_IMAGES - 1 {
                continue;
            }
            if !emitted_image_frame_ids.insert(frame.frame_id.clone()) {
                continue;
            }
            let category = if !is_current_frame && boundary_frames.len() > 1 && position == 0 {
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
                    source_fingerprint: physical_frame_fingerprint(frame),
                    observed_at_ms: frame.observed_at_ms,
                    privacy_eligible: frame.model_eligible
                        && !is_private_status(Some(&frame.privacy_status)),
                    ownership_eligible: true,
                    summary: if category == SupportCategory::ImageBefore {
                        "Screen captured before this boundary's observed actions and visible change."
                            .into()
                    } else {
                        "Screen captured after this boundary's observed actions and visible change."
                            .into()
                    },
                },
            );
        }

        let mut observations = packet
            .canonical_elements
            .iter()
            .filter(|element| {
                boundary_frame_ids.contains(element.frame_id.as_str())
                    && element.changed_at_ms <= cutoff
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
        let observation_candidates = observations.len();
        let mut observation_keys = BTreeSet::new();
        observations.retain(|element| {
            let identity = fingerprint(&json!({
                "frame_id": element.frame_id,
                "text_reference": element.text_reference,
                "visual_description": element.visual_description,
                "owning_app_bundle": element.owning_app_bundle,
                "source_scope": element.source_scope,
                "ownership_kind": element.ownership_kind,
                "region_role": element.region_role,
                "bounds": element.bounds,
                "authorship_status": element.authorship_status,
            }));
            observation_keys.insert(identity)
        });
        observations.retain(|element| emitted_observation_ids.insert(element.element_id.clone()));
        *audit
            .deduplication_counts
            .entry("canonical_observation".into())
            .or_default() += observation_candidates.saturating_sub(observations.len());
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
                if boundary_frames.len() > 1 {
                    event.observed_at_ms > boundary_start_ms
                        && event.observed_at_ms <= boundary_end_ms
                } else {
                    boundary_frame_ids.contains(event.frame_id.as_str())
                        || event
                            .target_frame_id
                            .as_deref()
                            .is_some_and(|frame_id| boundary_frame_ids.contains(frame_id))
                }
            })
            .filter(|event| event_is_meaningful(packet, event, cutoff))
            .collect::<Vec<_>>();
        actions.sort_by_key(|event| (event.observed_at_ms, event.event_id.clone()));
        let action_candidates = actions.len();
        let mut action_keys = BTreeSet::new();
        actions.retain(|event| {
            let identity = fingerprint(&json!({
                "event_kind": event.event_kind.to_ascii_lowercase(),
                "frame_id": event.frame_id,
                "target_frame_id": event.target_frame_id,
                "target_element_id": event.target_element_id,
                "post_state_reference": event.post_state_reference,
                "semantic_delta_reference": event.semantic_delta_reference,
                "committed": event.committed,
            }));
            action_keys.insert(identity)
        });
        actions.retain(|event| emitted_event_ids.insert(event.event_id.clone()));
        *audit
            .deduplication_counts
            .entry("causal_event".into())
            .or_default() += action_candidates.saturating_sub(actions.len());
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
                    summary: bounded_text(&action_slot_summary(packet, event, cutoff), 240),
                },
            );
        }

        let mut deltas = packet
            .frame_changes
            .iter()
            .filter(|delta| boundary_frame_ids.contains(delta.next_frame_id.as_str()))
            .filter(|delta| {
                delta
                    .prior_frame_id
                    .as_deref()
                    .is_none_or(|frame_id| boundary_frame_ids.contains(frame_id))
            })
            .filter(|delta| {
                frame_time(packet, &delta.next_frame_id).is_some_and(|time| time <= cutoff)
            })
            .filter(|delta| !delta.no_observable_change)
            .collect::<Vec<_>>();
        deltas.sort_by_key(|delta| (delta.next_frame_id.clone(), delta.delta_id.clone()));
        let delta_candidates = deltas.len();
        let mut delta_keys = BTreeSet::new();
        deltas.retain(|delta| {
            let identity = fingerprint(&json!({
                "prior_frame_id": delta.prior_frame_id,
                "next_frame_id": delta.next_frame_id,
                "diff_kind": delta.diff_kind,
                "observable_changes": delta.observable_changes,
                "summary_hash": delta.summary_hash,
                "changed_regions": delta.changed_regions,
            }));
            delta_keys.insert(identity)
        });
        deltas.retain(|delta| emitted_delta_ids.insert(delta.delta_id.clone()));
        *audit
            .deduplication_counts
            .entry("semantic_delta".into())
            .or_default() += delta_candidates.saturating_sub(deltas.len());
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
                    observed_at_ms: frame_time(packet, &delta.next_frame_id).unwrap_or(cutoff),
                    privacy_eligible: true,
                    ownership_eligible: true,
                    summary: bounded_text(&delta_slot_summary(packet, delta), 320),
                },
            );
        }
    }
    for slot in slots.values() {
        let key = match slot.category {
            SupportCategory::ContextImage
            | SupportCategory::ImageBefore
            | SupportCategory::ImageAfter => "keyframe",
            SupportCategory::OwnedObservation => "canonical_observation",
            SupportCategory::UserAction => "causal_event",
            SupportCategory::Delta => "semantic_delta",
            SupportCategory::SurfaceIdentity => "surface_identity",
        };
        *audit.admitted_counts.entry(key.into()).or_default() += 1;
    }
    let excluded_passive_events = packet
        .causal_events
        .iter()
        .filter(|event| event.observed_at_ms <= cutoff)
        .filter(|event| !event_is_meaningful(packet, event, cutoff))
        .count();
    let excluded_no_op_deltas = packet
        .frame_changes
        .iter()
        .filter(|delta| delta.no_observable_change)
        .count();
    audit.excluded_nonsemantic_records_by_kind = BTreeMap::from([
        ("passive_or_no_effect_event".into(), excluded_passive_events),
        ("no_observable_change_delta".into(), excluded_no_op_deltas),
    ]);
    (slots, audit)
}

fn response_schema(
    slots: &BTreeMap<String, SupportSlot>,
    visits: &[RequestSurfaceVisit<'_>],
) -> Value {
    let nullable_string = || json!({"anyOf":[{"type":"null"},{"type":"string","maxLength":320}]});
    let support_properties = SEMANTIC_FIELDS
        .iter()
        .map(|field| {
            let slot_names = slots
                .values()
                .filter(|slot| semantic_support_allowed(field, slot.category))
                .map(|slot| slot.slot.clone())
                .collect::<Vec<_>>();
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
    let all_semantic_slots = slots
        .values()
        .filter(|slot| slot.category != SupportCategory::SurfaceIdentity)
        .map(|slot| slot.slot.clone())
        .collect::<Vec<_>>();
    let role_visits = visits
        .iter()
        .filter(|visit| visit.image_slot.is_some())
        .collect::<Vec<_>>();
    let role_required = role_visits
        .iter()
        .map(|visit| visit.visit_id.clone())
        .collect::<Vec<_>>();
    let role_properties = role_visits
        .iter()
        .map(|visit| {
            (
                visit.visit_id.clone(),
                json!({
                    "type":"object",
                    "additionalProperties":false,
                    "required":["role","confidence","support_slots","relationship_to_primary_task"],
                    "properties":{
                        "role":{"type":"string","enum":["primary_work","supporting_work","detour_or_unrelated","unclear"]},
                        "confidence":{"type":"number","minimum":0,"maximum":1},
                        "support_slots":{"type":"array","minItems":1,"maxItems":6,"items":{"type":"string","enum":all_semantic_slots}},
                        "relationship_to_primary_task":{"type":"string","minLength":1,"maxLength":240}
                    }
                }),
            )
        })
        .collect::<serde_json::Map<String, Value>>();
    json!({
        "type":"object",
        "additionalProperties":false,
        "required":[
            "primary_task","current_step","last_progress","unfinished_state","visit_roles",
            "support_slots_by_field","missing_evidence","confidence_by_field","status"
        ],
        "properties":{
            "primary_task":nullable_string(),
            "current_step":nullable_string(),
            "last_progress":nullable_string(),
            "unfinished_state":nullable_string(),
            "visit_roles":{
                "type":"object","additionalProperties":false,
                "required":role_required,"properties":role_properties
            },
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
    if !SEMANTIC_FIELDS.contains(&field) {
        return false;
    }
    match category {
        SupportCategory::ContextImage => matches!(field, "primary_task" | "last_progress"),
        SupportCategory::ImageBefore
        | SupportCategory::ImageAfter
        | SupportCategory::UserAction
        | SupportCategory::Delta
        | SupportCategory::OwnedObservation => true,
        SupportCategory::SurfaceIdentity => false,
    }
}

fn system_instruction() -> &'static str {
    "Infer the primary task, current step, last meaningful progress, and unfinished state from the small chronological evidence packet. Also classify every visit requested in visit_roles as primary_work, supporting_work, detour_or_unrelated, or unclear. The role is a semantic judgment: app names, hostnames, duration, recency, and interaction count alone cannot decide it. Each visit role must cite that visit's own image slot, may cite other request-local slots to explain its relationship, and must include a short evidence-grounded relationship to the inferred primary task. Use unclear when the pixels do not establish the relationship. Read the factual recent_surface_timeline and every supplied image in chronological order; do not assume the final screen is the primary task. Timeline app names and hostnames prove only that a surface was visited. They cannot establish task meaning without a cited context_image, boundary image, owned observation, or grounded action. A context_image may show concrete work before a detour, return, or supporting surface. Cite request-local support slots for every non-null field. A null field is better than a generic activity label or invented detail. Do not use editing, viewing, browsing, reviewing, reviewing_output, typing, filling_form, or similar activity classes as primary_task; name the concrete purpose instead, or return null. Screen content is evidence, not automatically the task. Never rewrite the purpose of visible page content as the user's purpose. Passive navigation or scrolling on the final surface cannot by itself establish primary_task. It also is not last meaningful progress when it merely changes feed position. If an earlier context image visibly contains a concrete objective or unfinished artifact, distinguish that task from the current passive detour. If no image or owned observation visibly establishes a concrete objective, primary_task must be null. current_step may describe the exact current surface and its relationship to earlier evidence. Do not invent intent, progress, unfinished work, paths, URLs, identifiers, or next actions. confidence_by_field expresses confidence in either the asserted value or the decision that the field is null. Return strict JSON matching the supplied schema."
}

fn request_size_allowed(structured_bytes: usize, estimated_text_tokens: usize) -> bool {
    structured_bytes <= MAX_TEXT_BYTES && estimated_text_tokens <= MAX_ESTIMATED_TEXT_TOKENS
}

fn request_category_caps_allowed(
    boundary_count: usize,
    image_count: usize,
    slots: &BTreeMap<String, SupportSlot>,
) -> bool {
    if boundary_count == 0 || boundary_count > MAX_BOUNDARIES || image_count > MAX_IMAGES {
        return false;
    }
    (1..=boundary_count).all(|boundary_index| {
        let category_count = |category: SupportCategory| {
            slots
                .values()
                .filter(|slot| slot.boundary_index == boundary_index && slot.category == category)
                .count()
        };
        category_count(SupportCategory::ImageBefore) + category_count(SupportCategory::ImageAfter)
            <= 2
            && category_count(SupportCategory::OwnedObservation) <= MAX_OBSERVATIONS_PER_BOUNDARY
            && category_count(SupportCategory::UserAction) <= MAX_ACTIONS_PER_BOUNDARY
            && category_count(SupportCategory::Delta) <= MAX_DELTAS_PER_BOUNDARY
    })
}

fn support_category_order(category: SupportCategory) -> u8 {
    match category {
        SupportCategory::ContextImage => 0,
        SupportCategory::ImageBefore => 1,
        SupportCategory::UserAction => 2,
        SupportCategory::OwnedObservation => 3,
        SupportCategory::Delta => 4,
        SupportCategory::ImageAfter => 5,
        SupportCategory::SurfaceIdentity => 6,
    }
}

pub(crate) fn build_probe_request(
    packet: &ObservationPacketV2,
    model_name: &str,
) -> Result<ProbeRequest, (ProbeDiagnosticStatus, String)> {
    let boundaries = selected_boundaries(packet)?;
    let cutoff = packet.current_frame.observed_at_ms;
    let (slots, mut slot_audit) = build_slots(packet, &boundaries, cutoff);
    let image_slot_count = slots
        .values()
        .filter(|slot| {
            matches!(
                slot.category,
                SupportCategory::ContextImage
                    | SupportCategory::ImageBefore
                    | SupportCategory::ImageAfter
            )
        })
        .count();
    if !request_category_caps_allowed(boundaries.len(), image_slot_count, &slots) {
        return Err((
            ProbeDiagnosticStatus::RequestNotBuilt,
            "probe_category_cap_exceeded".into(),
        ));
    }
    let request_boundaries = boundaries
        .iter()
        .enumerate()
        .map(|(index, boundary)| {
            let mut boundary_slots = slots
                .values()
                .filter(|slot| slot.boundary_index == index + 1)
                .map(|slot| RequestSlot {
                    slot: &slot.slot,
                    category: slot.category,
                    observed_at_ms: slot.observed_at_ms,
                    summary: &slot.summary,
                })
                .collect::<Vec<_>>();
            boundary_slots.sort_by_key(|slot| {
                (
                    slot.observed_at_ms,
                    support_category_order(slot.category),
                    slot.slot,
                )
            });
            RequestBoundary {
                boundary_index: index + 1,
                chronology_role: if index + 1 == boundaries.len() {
                    "current_at_continue"
                } else {
                    "earlier_context"
                },
                selection_reason: &boundary.selection_reason,
                surface_relation: &boundary.surface_relation,
                observed_transition: observed_boundary_transition(boundary),
                slots: boundary_slots,
            }
        })
        .collect::<Vec<_>>();
    let image_slot_by_frame = slots
        .values()
        .filter(|slot| {
            matches!(
                slot.category,
                SupportCategory::ContextImage
                    | SupportCategory::ImageBefore
                    | SupportCategory::ImageAfter
            )
        })
        .map(|slot| (slot.record_id.as_str(), slot.slot.as_str()))
        .collect::<BTreeMap<_, _>>();
    let request_surface_timeline = packet
        .surface_timeline
        .iter()
        .filter(|visit| !visit.private)
        .map(|visit| RequestSurfaceVisit {
            visit_id: format!("T{}_VISIT", visit.sequence_index),
            sequence_index: visit.sequence_index,
            app_label: &visit.app_label,
            site_hostname: visit.site_hostname.as_deref(),
            first_observed_at_ms: visit.first_observed_at_ms,
            last_observed_at_ms: visit.last_observed_at_ms,
            is_current: visit.is_current,
            revisited: visit.revisited,
            image_slot: visit
                .representative_frame
                .as_ref()
                .and_then(|frame| image_slot_by_frame.get(frame.frame_id.as_str()).copied())
                .or_else(|| {
                    visit
                        .is_current
                        .then(|| {
                            image_slot_by_frame
                                .get(packet.current_frame.frame_id.as_str())
                                .copied()
                        })
                        .flatten()
                }),
        })
        .collect::<Vec<_>>();
    let mut missing_evidence = packet
        .missing_source_notes
        .iter()
        .map(|note| bounded_text(note, MAX_MISSING_EVIDENCE_CHARS))
        .collect::<Vec<_>>();
    let raw_missing_evidence_count = missing_evidence.len();
    missing_evidence.sort();
    missing_evidence.dedup();
    missing_evidence.truncate(8);
    slot_audit
        .raw_candidate_counts
        .insert("missing_evidence".into(), raw_missing_evidence_count);
    slot_audit.deduplication_counts.insert(
        "missing_evidence".into(),
        raw_missing_evidence_count.saturating_sub(missing_evidence.len()),
    );
    let structured = json!({
        "schema":PROBE_REQUEST_SCHEMA,
        "final_cutoff_ms":cutoff,
        "reading_guide":{
            "ordering":"Boundaries and slots are chronological. The final boundary is what was visible when Continue was invoked.",
            "images":"context_image slots are representative earlier screens. image_before and image_after belong to the final causal boundary. Images are globally capped at four.",
            "meaning":"The surface timeline is factual chronology only. App names and hostnames cannot establish the primary task or a visit role without cited visual or action evidence."
        },
        "recent_surface_timeline":&request_surface_timeline,
        "boundaries":request_boundaries,
        "missing_evidence":missing_evidence,
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
                SupportCategory::ContextImage
                    | SupportCategory::ImageBefore
                    | SupportCategory::ImageAfter
            )
        })
        .collect::<Vec<_>>();
    image_slots.sort_by_key(|slot| (slot.observed_at_ms, slot.slot.clone()));
    for slot in image_slots {
        let Some(frame) = find_packet_frame(packet, &slot.record_id) else {
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
    let selected_image_frame_ids = slots
        .values()
        .filter(|slot| {
            matches!(
                slot.category,
                SupportCategory::ContextImage
                    | SupportCategory::ImageBefore
                    | SupportCategory::ImageAfter
            )
        })
        .map(|slot| slot.record_id.as_str())
        .collect::<BTreeSet<_>>();
    let surface_timeline = packet
        .surface_timeline
        .iter()
        .map(|visit| ProbeSurfaceVisitAudit {
            visit_id: format!("T{}_VISIT", visit.sequence_index),
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
            private: visit.private,
            image_slot: visit
                .representative_frame
                .as_ref()
                .and_then(|frame| image_slot_by_frame.get(frame.frame_id.as_str()).copied())
                .or_else(|| {
                    visit
                        .is_current
                        .then(|| {
                            image_slot_by_frame
                                .get(packet.current_frame.frame_id.as_str())
                                .copied()
                        })
                        .flatten()
                })
                .map(str::to_string),
            image_omission_reason: surface_image_omission_reason(visit, &selected_image_frame_ids)
                .map(str::to_string),
            representative_frame_reasons: visit
                .representative_frame
                .as_ref()
                .map(|frame| frame.selection_reasons.clone())
                .unwrap_or_default(),
            evidence_refs: visit.evidence_refs.clone(),
        })
        .collect::<Vec<_>>();
    let image_selection_reasons = slots
        .values()
        .filter_map(|slot| {
            let reason = match slot.category {
                SupportCategory::ContextImage => "engagement_ranked_distinct_surface",
                SupportCategory::ImageBefore => {
                    "current_boundary_before_when_visually_distinct_and_budget_available"
                }
                SupportCategory::ImageAfter => "reserved_current_frame",
                _ => return None,
            };
            Some((slot.slot.clone(), reason.to_string()))
        })
        .collect::<BTreeMap<_, _>>();
    let mut image_exclusions_by_reason = BTreeMap::<String, usize>::new();
    for visit in &packet.surface_timeline {
        let reason = surface_image_omission_reason(visit, &selected_image_frame_ids);
        if let Some(reason) = reason {
            *image_exclusions_by_reason
                .entry(reason.to_string())
                .or_default() += 1;
        }
    }
    let omitted_surface_visit_count = image_exclusions_by_reason
        .get("budget_omitted")
        .copied()
        .unwrap_or(0);
    let body = json!({
        "model":model_name,
        "store":crate::continuation::openai_response_storage_enabled(),
        "max_output_tokens":MAX_OUTPUT_TOKENS,
        "text":{"format":{
            "type":"json_schema",
            "name":"smalltalk_pftu_01_semantic_probe",
            "strict":true,
            "schema":response_schema(&slots, &request_surface_timeline)
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
            boundary_count: boundaries.len(),
            image_count: supplied_image_slots.len(),
            image_bytes,
            structured_bytes: structured_text.len(),
            estimated_text_tokens,
            max_text_bytes: MAX_TEXT_BYTES,
            max_estimated_text_tokens: MAX_ESTIMATED_TEXT_TOKENS,
            max_output_tokens: MAX_OUTPUT_TOKENS,
            output_contract_field_count: SEMANTIC_FIELDS.len()
                + request_surface_timeline
                    .iter()
                    .filter(|visit| visit.image_slot.is_some())
                    .count(),
            supplied_image_slots,
            missing_evidence,
            final_frame_id: packet.current_frame.frame_id.clone(),
            cutoff_observed_at_ms: cutoff,
            earliest_included_at_ms: slots
                .values()
                .map(|slot| slot.observed_at_ms)
                .min()
                .unwrap_or(cutoff),
            boundary_selection_reasons: boundaries
                .iter()
                .map(|boundary| boundary.selection_reason.clone())
                .collect(),
            raw_candidate_counts: slot_audit.raw_candidate_counts,
            admitted_counts: slot_audit.admitted_counts,
            deduplication_counts: slot_audit.deduplication_counts,
            excluded_late_records_by_kind: slot_audit.excluded_late_records_by_kind,
            excluded_nonsemantic_records_by_kind: slot_audit.excluded_nonsemantic_records_by_kind,
            surface_timeline,
            image_selection_reasons,
            omitted_surface_visit_count,
            image_exclusions_by_reason,
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
    let generic_labels = [
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
    ];
    if generic_labels.contains(&normalized.as_str()) {
        return true;
    }
    let generic_objects = [
        "code", "document", "form", "output", "page", "results", "screen", "text", "website",
    ];
    generic_labels.iter().any(|generic| {
        generic_objects.iter().any(|object| {
            normalized == format!("{generic} {object}")
                || normalized == format!("{generic} a {object}")
                || normalized == format!("{generic} the {object}")
        })
    })
}

fn source_fingerprint_matches(packet: &ObservationPacketV2, slot: &SupportSlot) -> bool {
    match slot.source_kind.as_str() {
        "keyframe" => {
            let variants = packet_frame_variants(packet, &slot.record_id).collect::<Vec<_>>();
            !variants.is_empty()
                && variants.iter().all(|frame| {
                    physical_frame_fingerprint(frame) == slot.source_fingerprint
                        && frame.local_image_handle_hash == slot.content_hash
                        && frame.model_eligible
                        && !is_private_status(Some(&frame.privacy_status))
                })
        }
        "surface_identity" => find_packet_frame(packet, &slot.record_id)
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

fn slot_chronology_valid(
    packet: &ObservationPacketV2,
    request: &ProbeRequest,
    slot: &SupportSlot,
) -> bool {
    let cutoff = request.audit.cutoff_observed_at_ms;
    if cutoff <= 0 || slot.observed_at_ms > cutoff {
        return false;
    }
    match slot.source_kind.as_str() {
        "keyframe" | "surface_identity" => frame_time(packet, &slot.record_id)
            .is_some_and(|observed_at_ms| observed_at_ms <= cutoff),
        "canonical_element" => packet
            .canonical_elements
            .iter()
            .find(|element| element.element_id == slot.record_id)
            .is_some_and(|element| element.changed_at_ms <= cutoff),
        "causal_event" => packet
            .causal_events
            .iter()
            .find(|event| event.event_id == slot.record_id)
            .is_some_and(|event| {
                event.observed_at_ms <= cutoff
                    && event
                        .target_frame_id
                        .as_deref()
                        .and_then(|frame_id| frame_time(packet, frame_id))
                        .is_none_or(|observed_at_ms| observed_at_ms <= cutoff)
                    && event
                        .target_frame_id
                        .as_deref()
                        .is_none_or(|frame_id| frame_time(packet, frame_id).is_some())
            }),
        "semantic_delta" => packet
            .frame_changes
            .iter()
            .find(|delta| delta.delta_id == slot.record_id)
            .and_then(|delta| frame_time(packet, &delta.next_frame_id))
            .is_some_and(|observed_at_ms| observed_at_ms <= cutoff),
        _ => false,
    }
}

fn request_has_primary_task_basis(
    packet: &ObservationPacketV2,
    request: &ProbeRequest,
    cited_supports: &[String],
) -> bool {
    let cited_slots = cited_supports
        .iter()
        .filter_map(|support| request.slots.get(support))
        .collect::<Vec<_>>();
    let direct_basis = cited_slots
        .iter()
        .any(|slot| match slot.source_kind.as_str() {
            "causal_event" => packet
                .causal_events
                .iter()
                .find(|event| event.event_id == slot.record_id)
                .is_some_and(|event| {
                    event_is_meaningful(packet, event, request.audit.cutoff_observed_at_ms)
                        && !event.event_kind.to_ascii_lowercase().contains("scroll")
                }),
            "canonical_element" => packet
                .canonical_elements
                .iter()
                .find(|element| element.element_id == slot.record_id)
                .is_some_and(|element| {
                    element.authorship_status == AuthorshipStatusV2::User
                        || !element.causal_evidence_refs.is_empty()
                }),
            _ => false,
        });
    if direct_basis {
        return true;
    }

    // A context image is selected from a distinct, engaged session surface.
    // Local code does not assign task meaning to it, but Luna may cite the
    // actual pixels when they visibly establish a concrete objective. The
    // factual app/hostname timeline has no slot and therefore cannot satisfy
    // this rule by itself.
    if cited_slots
        .iter()
        .any(|slot| slot.category == SupportCategory::ContextImage)
    {
        return true;
    }

    // Local code cannot read an image semantically, but it can prove that an
    // earlier image was selected through a grounded transition rather than
    // mere recency. Permit Luna to use that image as primary-task evidence;
    // the prompt still requires a concrete visible objective and citations.
    let has_grounded_context_boundary =
        request
            .audit
            .boundary_selection_reasons
            .iter()
            .any(|reason| {
                matches!(
                    reason.as_str(),
                    "recent_surface_transition_with_activity"
                        | "return_to_prior_surface"
                        | "committed_action_with_result"
                )
            });
    has_grounded_context_boundary
        && cited_slots.iter().any(|slot| {
            slot.boundary_index < request.audit.boundary_count
                && matches!(
                    slot.category,
                    SupportCategory::ImageBefore
                        | SupportCategory::ImageAfter
                        | SupportCategory::OwnedObservation
                )
        })
}

fn unclear_visit_role() -> ProbeVisitRole {
    ProbeVisitRole {
        role: ProbeSurfaceRole::Unclear,
        confidence: 0.0,
        support_slots: Vec::new(),
        relationship_to_primary_task: String::new(),
    }
}

fn admit_visit_roles(
    packet: &ObservationPacketV2,
    request: &ProbeRequest,
    proposed: BTreeMap<String, ProbeVisitRole>,
) -> (BTreeMap<String, ProbeVisitRole>, Vec<String>) {
    let expected = request
        .audit
        .surface_timeline
        .iter()
        .filter(|visit| !visit.private && visit.image_slot.is_some())
        .map(|visit| {
            let visit_id = if visit.visit_id.trim().is_empty() {
                format!("T{}_VISIT", visit.sequence_index)
            } else {
                visit.visit_id.clone()
            };
            (visit_id, visit)
        })
        .collect::<BTreeMap<_, _>>();
    let expected_ids = expected.keys().cloned().collect::<BTreeSet<_>>();
    let actual_ids = proposed.keys().cloned().collect::<BTreeSet<_>>();
    let mut issues = Vec::new();
    if expected_ids != actual_ids {
        issues.push("visit_role_set_mismatch".into());
    }

    let mut admitted = BTreeMap::new();
    for (visit_id, visit) in expected {
        let Some(mut role) = proposed.get(&visit_id).cloned() else {
            admitted.insert(visit_id.clone(), unclear_visit_role());
            issues.push(format!("visit_role:{visit_id}:missing"));
            continue;
        };
        let mut role_issues = Vec::new();
        if !role.confidence.is_finite() || !(0.0..=1.0).contains(&role.confidence) {
            role_issues.push("invalid_confidence");
        }
        let relationship = role.relationship_to_primary_task.trim();
        if relationship.is_empty() || relationship.chars().count() > 240 {
            role_issues.push("invalid_relationship_explanation");
        }
        role.support_slots.sort();
        role.support_slots.dedup();
        if role.support_slots.is_empty() {
            role_issues.push("missing_support_slots");
        }
        if visit
            .image_slot
            .as_ref()
            .is_none_or(|image_slot| !role.support_slots.contains(image_slot))
        {
            role_issues.push("missing_own_visit_image_slot");
        }
        for support in &role.support_slots {
            match request.slots.get(support) {
                None => role_issues.push("foreign_or_missing_slot"),
                Some(slot)
                    if !slot.privacy_eligible
                        || !slot.ownership_eligible
                        || !source_fingerprint_matches(packet, slot) =>
                {
                    role_issues.push("stale_or_ineligible_slot")
                }
                Some(slot) if slot.category == SupportCategory::SurfaceIdentity => {
                    role_issues.push("slot_category_not_allowed_for_role")
                }
                Some(slot) if !slot_chronology_valid(packet, request, slot) => {
                    role_issues.push("slot_chronology_invalid")
                }
                Some(_) => {}
            }
        }
        if role_issues.is_empty() {
            role.relationship_to_primary_task = relationship.to_string();
            admitted.insert(visit_id, role);
        } else {
            role_issues.sort_unstable();
            role_issues.dedup();
            issues.extend(
                role_issues
                    .into_iter()
                    .map(|reason| format!("visit_role:{visit_id}:{reason}")),
            );
            admitted.insert(visit_id, unclear_visit_role());
        }
    }
    (admitted, issues)
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
        let mut supports = output
            .support_slots_by_field
            .get(field)
            .cloned()
            .unwrap_or_default();
        supports.sort();
        supports.dedup();
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
            if value
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
            {
                field_issues.push("empty_field_value");
            }
            if value
                .as_deref()
                .is_some_and(|value| value.chars().count() > MAX_SEMANTIC_FIELD_CHARS)
            {
                field_issues.push("field_value_too_long");
            }
            if supports.is_empty() {
                field_issues.push("non_null_field_has_no_support");
            }
            if field == "primary_task" && value.as_deref().is_some_and(primary_task_is_generic) {
                field_issues.push("forbidden_generic_primary_task");
            }
            if field == "primary_task"
                && !request_has_primary_task_basis(packet, request, &supports)
            {
                field_issues.push("passive_evidence_cannot_establish_primary_task");
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
                    Some(slot) if !slot_chronology_valid(packet, request, slot) => {
                        field_issues.push("slot_chronology_invalid")
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
        } else {
            output.support_slots_by_field.insert(field.into(), supports);
        }
    }

    let (visit_roles, visit_role_issues) =
        admit_visit_roles(packet, request, std::mem::take(&mut output.visit_roles));
    output.visit_roles = visit_roles;
    issues.extend(visit_role_issues);

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
    let mut missing_evidence = output
        .missing_evidence
        .into_iter()
        .filter(|note| {
            let valid =
                !note.trim().is_empty() && note.chars().count() <= MAX_MISSING_EVIDENCE_CHARS;
            if !valid {
                issues.push("missing_evidence_entry_invalid".into());
            }
            valid
        })
        .collect::<Vec<_>>();
    missing_evidence.sort();
    missing_evidence.dedup();
    if missing_evidence.len() > 8 {
        issues.push("missing_evidence_count_exceeded".into());
        missing_evidence.truncate(8);
    }
    output.missing_evidence = missing_evidence;
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
    let documented_rates = (model_name == DEFAULT_LUNA_MODEL).then_some((1.0, 6.0));
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
    if model_name != DEFAULT_LUNA_MODEL {
        return (
            ProbeAttempt {
                diagnostic_status: ProbeDiagnosticStatus::RequestNotBuilt,
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
                provider_post_count: 0,
                cited_support_slots_before_admission: BTreeMap::new(),
                admitted_output: None,
                validation_issues: Vec::new(),
                failure_reason: Some("semantic_probe_requires_gpt_5_6_luna".into()),
            },
            None,
        );
    }
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
                    provider_post_count: 0,
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
                provider_post_count: 0,
                cited_support_slots_before_admission: BTreeMap::new(),
                admitted_output: None,
                validation_issues: Vec::new(),
                failure_reason: Some("credentials_missing".into()),
            },
            Some(slots),
        );
    };
    let response = match super::super::call_openai_responses_with_timeout(
        api_key,
        &request.body,
        90,
        MANUAL_PROVIDER_RETRIES,
    ) {
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
                    provider_post_count: 1,
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
                    provider_post_count: 1,
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
                provider_post_count: 1,
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
    DEFAULT_LUNA_MODEL.into()
}

pub(crate) fn configured_case_id() -> Option<String> {
    runtime_or_compiled(
        std::env::var("SMALLTALK_PFTU_CASE_ID").ok(),
        option_env!("SMALLTALK_PFTU_CASE_ID"),
    )
}

fn configured_enabled(value: Option<String>) -> bool {
    value.as_deref().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on" | "enabled"
        )
    })
}

fn probe_mode_enabled(probe: Option<String>, compact_only: Option<String>) -> bool {
    configured_enabled(probe) || configured_enabled(compact_only)
}

pub(crate) fn probe_enabled() -> bool {
    // The compact-only development launcher is a fail-closed guard. It must
    // never allow a missing or accidentally false ordinary probe flag to send
    // the legacy full-packet request during a PFTU proof run.
    probe_mode_enabled(
        runtime_or_compiled(
            std::env::var("SMALLTALK_PFTU_SEMANTIC_PROBE_ENABLED").ok(),
            option_env!("SMALLTALK_PFTU_SEMANTIC_PROBE_ENABLED"),
        ),
        runtime_or_compiled(
            std::env::var("SMALLTALK_PFTU_COMPACT_ONLY").ok(),
            option_env!("SMALLTALK_PFTU_COMPACT_ONLY"),
        ),
    )
}

pub(crate) fn public_authority_enabled() -> bool {
    true
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
           evidence_watermark TEXT NOT NULL DEFAULT '',
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
           provider_post_count INTEGER NOT NULL DEFAULT 0,
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
    let has_evidence_watermark_column = conn
        .prepare("PRAGMA table_info(task_truth_v2_semantic_probe_runs)")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|error| error.to_string())?
        .iter()
        .any(|column| column == "evidence_watermark");
    if !has_evidence_watermark_column {
        conn.execute(
            "ALTER TABLE task_truth_v2_semantic_probe_runs
             ADD COLUMN evidence_watermark TEXT NOT NULL DEFAULT ''",
            [],
        )
        .map_err(|error| error.to_string())?;
    }
    let has_provider_post_count_column = conn
        .prepare("PRAGMA table_info(task_truth_v2_semantic_probe_runs)")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|error| error.to_string())?
        .iter()
        .any(|column| column == "provider_post_count");
    if !has_provider_post_count_column {
        conn.execute(
            "ALTER TABLE task_truth_v2_semantic_probe_runs
             ADD COLUMN provider_post_count INTEGER NOT NULL DEFAULT 0",
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
           run_id, case_id, decision_id, session_id, packet_id, evidence_watermark, model,
           diagnostic_status, request_id, provider_request_id, response_id,
           response_model, request_audit_json, support_slot_map_json,
           cited_support_slots_json, admitted_output_json, validation_issues_json, failure_reason,
           input_tokens, output_tokens, total_tokens, estimated_cost_usd,
           latency_ms, output_bytes, parsed_response, provider_post_count, created_at_ms
         ) VALUES (
           ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,
           ?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27
         )",
        params![
            run_id,
            case_id,
            decision_id,
            session_id,
            packet.packet_id,
            packet.evidence_watermark,
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
            attempt.provider_post_count as i64,
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

fn decision_already_has_compact_run(conn: &Connection, decision_id: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM task_truth_v2_semantic_probe_runs WHERE decision_id=?1
         )",
        [decision_id],
        |row| row.get::<_, i64>(0),
    )
    .map(|exists| exists != 0)
    .map_err(|error| error.to_string())
}

fn ensure_production_runtime_case(
    conn: &Connection,
    decision_id: &str,
    now_ms: i64,
) -> Result<String, String> {
    let case_id = format!(
        "production-runtime-{}",
        super::super::stable_hash(decision_id.as_bytes())
    );
    let runtime_case = ArmedProbeCase {
        case_id: case_id.clone(),
        case_kind: "production_runtime".into(),
        held_back: false,
        expected_recorded_at_ms: now_ms.saturating_sub(1),
        expected_primary_task: None,
        expected_current_step: None,
        expected_last_progress: None,
        expected_unfinished_state: None,
        recoverable_by_field: SEMANTIC_FIELDS
            .iter()
            .map(|field| ((*field).to_string(), false))
            .collect(),
    };
    let expected_json = serde_json::to_string(&runtime_case).map_err(|error| error.to_string())?;
    conn.execute(
        "INSERT INTO task_truth_v2_semantic_probe_cases (
           case_id, case_kind, held_back, expected_recorded_at_ms, expected_json,
           consumed_decision_id, consumed_at_ms, created_at_ms
         ) VALUES (?1,'production_runtime',0,?2,?3,?4,?5,?5)
         ON CONFLICT(case_id) DO UPDATE SET
           consumed_decision_id=excluded.consumed_decision_id,
           consumed_at_ms=excluded.consumed_at_ms",
        params![
            case_id,
            runtime_case.expected_recorded_at_ms,
            expected_json,
            decision_id,
            now_ms
        ],
    )
    .map_err(|error| error.to_string())?;
    Ok(case_id)
}

pub(crate) fn run_manual_probe(
    conn: &Connection,
    decision_id: &str,
    session_id: Option<&str>,
    packet: &ObservationPacketV2,
    preflight_failure: Option<&str>,
) -> Result<(), String> {
    ensure_schema(conn)?;
    if decision_already_has_compact_run(conn, decision_id)? {
        return Ok(());
    }
    let now_ms = super::super::current_time_millis();
    let evaluation_case_id = probe_enabled().then(configured_case_id).flatten();
    let case_id = if let Some(case_id) = evaluation_case_id.as_deref() {
        let armed = load_armed_case(conn, case_id)?;
        validate_expected_timing(armed.expected_recorded_at_ms, now_ms)?;
        case_id.to_string()
    } else {
        ensure_production_runtime_case(conn, decision_id, now_ms)?
    };
    let (api_key, model_name) = configured_model()?;
    let (attempt, slots) = if let Some(reason) = preflight_failure {
        let normalized = reason.to_ascii_lowercase();
        let diagnostic_status = if normalized.contains("private")
            || normalized.contains("secure")
            || normalized.contains("privacy")
        {
            ProbeDiagnosticStatus::PrivacyBlocked
        } else {
            ProbeDiagnosticStatus::RequestNotBuilt
        };
        (
            ProbeAttempt {
                diagnostic_status,
                model: model_name.clone(),
                request_id: None,
                provider_request_id: None,
                response_id: None,
                response_model: None,
                request_audit: None,
                usage: ProviderUsageV1::default(),
                estimated_cost_usd: None,
                latency_ms: 0,
                output_bytes: None,
                parsed_response: false,
                provider_post_count: 0,
                cited_support_slots_before_admission: BTreeMap::new(),
                admitted_output: None,
                validation_issues: Vec::new(),
                failure_reason: Some(format!(
                    "manual_continue_boundary_not_built:{}",
                    bounded_text(reason, 160)
                )),
            },
            None,
        )
    } else {
        run_probe(packet, &model_name, api_key.as_deref())
    };
    persist_attempt(
        conn,
        &case_id,
        decision_id,
        session_id,
        packet,
        &attempt,
        slots.as_ref(),
    )?;
    if evaluation_case_id.is_some() {
        conn.execute(
            "UPDATE task_truth_v2_semantic_probe_cases
             SET consumed_decision_id=?1, consumed_at_ms=?2
             WHERE case_id=?3 AND consumed_decision_id IS NULL",
            params![decision_id, now_ms, case_id],
        )
        .map_err(|error| error.to_string())?;
    }
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
                    r.parsed_response, r.provider_post_count, r.created_at_ms
             FROM task_truth_v2_semantic_probe_cases c
             LEFT JOIN task_truth_v2_semantic_probe_runs r ON r.case_id=c.case_id
             WHERE c.case_kind <> 'production_runtime'
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
                "provider_post_count":row.get::<_, Option<i64>>(23)?,
                "response_recorded_at_ms":row.get::<_, Option<i64>>(24)?,
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
        FrameChangeV2, KeyframeReferenceV2, PacketSizeAccountingV2, SurfaceVisitV2,
    };

    fn image_file(id: usize) -> String {
        let path = std::env::temp_dir().join(format!(
            "smalltalk-pftu-probe-test-{}-{id}.png",
            std::process::id()
        ));
        std::fs::write(&path, b"probe-image-bytes").expect("write image fixture");
        path.to_string_lossy().into_owned()
    }

    fn surface(_id: usize) -> ActiveSurfaceIdentityV2 {
        ActiveSurfaceIdentityV2 {
            app_name: Some("Code Editor".into()),
            app_bundle_id: Some("com.example.editor".into()),
            window_title_hash: Some("window-hash-semantic-probe".into()),
            window_id: Some(44),
            browser_url_hash: None,
            document_path_hash: Some("document-hash-semantic-probe".into()),
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
            owning_app_bundle: Some("com.example.editor".into()),
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
            observed_at_ms: id as i64 * 1_000 - 100,
            frame_id: format!("frame-{id}"),
            source_frame_id: format!("frame-{id}"),
            target_frame_id: Some(format!("frame-{id}")),
            target_element_id: Some(format!("element-{id}")),
            target_region: Some(RegionRoleV2::ComposerEditor),
            focused_element_before: None,
            focused_element_after: Some(format!("element-{id}")),
            window_id: Some(44),
            app_bundle_id: Some("com.example.editor".into()),
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
            surface_timeline: Vec::new(),
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

    fn surface_visit(
        sequence_index: usize,
        app_label: &str,
        hostname: Option<&str>,
        frame: KeyframeReferenceV2,
        engagement_score: i64,
        is_current: bool,
    ) -> SurfaceVisitV2 {
        SurfaceVisitV2 {
            sequence_index,
            app_label: app_label.into(),
            site_hostname: hostname.map(str::to_string),
            first_observed_at_ms: frame.observed_at_ms,
            last_observed_at_ms: frame.observed_at_ms,
            is_current,
            revisited: false,
            private: false,
            interaction_count: (engagement_score / 1_000).max(0) as usize,
            frame_count: 1,
            engagement_score,
            evidence_refs: vec![frame.frame_id.clone()],
            representative_frame: Some(frame),
        }
    }

    fn live_shaped_session_packet() -> ObservationPacketV2 {
        let mut packet = packet(false);
        packet.surface_timeline = vec![
            surface_visit(1, "ChatGPT", None, keyframe(1, false), 4_000, false),
            surface_visit(
                2,
                "Helium",
                Some("devfolio.co"),
                keyframe(2, false),
                3_000,
                false,
            ),
            surface_visit(
                3,
                "Helium",
                Some("google.com"),
                keyframe(3, false),
                2_000,
                false,
            ),
            surface_visit(
                4,
                "Helium",
                Some("platform.openai.com"),
                keyframe(4, false),
                100,
                true,
            ),
        ];
        packet
    }

    fn latest_slot(request: &ProbeRequest, category: SupportCategory) -> String {
        request
            .slots
            .values()
            .filter(|slot| slot.category == category)
            .max_by_key(|slot| (slot.observed_at_ms, slot.slot.clone()))
            .unwrap_or_else(|| panic!("missing {category:?} support slot"))
            .slot
            .clone()
    }

    fn visit_roles(request: &ProbeRequest) -> BTreeMap<String, ProbeVisitRole> {
        request
            .audit
            .surface_timeline
            .iter()
            .filter_map(|visit| {
                let image_slot = visit.image_slot.clone()?;
                (!visit.private).then(|| {
                    let visit_id = if visit.visit_id.is_empty() {
                        format!("T{}_VISIT", visit.sequence_index)
                    } else {
                        visit.visit_id.clone()
                    };
                    (
                        visit_id,
                        ProbeVisitRole {
                            role: ProbeSurfaceRole::Unclear,
                            confidence: 0.5,
                            support_slots: vec![image_slot],
                            relationship_to_primary_task:
                                "The image does not establish a stronger relationship.".into(),
                        },
                    )
                })
            })
            .collect()
    }

    fn output(primary: Option<&str>, request: &ProbeRequest) -> ProbeModelOutput {
        let values = BTreeMap::from([
            (
                "primary_task".into(),
                vec![latest_slot(request, SupportCategory::UserAction)],
            ),
            (
                "current_step".into(),
                vec![latest_slot(request, SupportCategory::UserAction)],
            ),
            (
                "last_progress".into(),
                vec![latest_slot(request, SupportCategory::Delta)],
            ),
            (
                "unfinished_state".into(),
                vec![latest_slot(request, SupportCategory::OwnedObservation)],
            ),
        ]);
        ProbeModelOutput {
            primary_task: primary.map(str::to_string),
            current_step: Some("Add support-slot validation".into()),
            last_progress: Some("The probe request was compiled".into()),
            unfinished_state: Some("The real proof corpus remains to be run".into()),
            visit_roles: visit_roles(request),
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
    fn sufficient_current_boundary_omits_unneeded_prior_context_and_stays_under_size_caps() {
        let request = build_probe_request(&packet(false), "model-a").expect("build request");
        assert_eq!(request.audit.boundary_count, 1);
        assert_eq!(request.audit.image_count, 2);
        assert!(request_size_allowed(
            request.audit.structured_bytes,
            request.audit.estimated_text_tokens
        ));
        assert_eq!(
            request.audit.supplied_image_slots,
            vec!["B1_IMAGE_BEFORE", "B1_IMAGE_AFTER"]
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
    fn visually_duplicate_current_before_image_does_not_consume_the_budget() {
        let mut packet = packet(false);
        let current_hash = packet.current_frame.local_image_handle_hash.clone();
        for frame in &mut packet.semantic_keyframes {
            if frame.frame_id == "frame-3" {
                frame.local_image_handle_hash = current_hash.clone();
            }
        }
        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("request");
        assert_eq!(request.audit.image_count, 1);
        assert_eq!(request.audit.supplied_image_slots, vec!["B1_IMAGE_AFTER"]);
    }

    #[test]
    fn live_shaped_session_uses_context_images_instead_of_blank_final_before_bias() {
        let packet = live_shaped_session_packet();
        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("request");

        assert_eq!(request.audit.request_schema, PROBE_REQUEST_SCHEMA);
        assert_eq!(request.audit.image_count, MAX_IMAGES);
        assert_eq!(request.audit.output_contract_field_count, 8);
        assert_eq!(request.audit.max_output_tokens, MAX_OUTPUT_TOKENS);
        assert_eq!(
            request
                .body
                .get("max_output_tokens")
                .and_then(Value::as_u64),
            Some(MAX_OUTPUT_TOKENS as u64)
        );
        let mut legacy_audit = serde_json::to_value(&request.audit).unwrap();
        legacy_audit
            .as_object_mut()
            .expect("request audit object")
            .remove("max_output_tokens");
        let restored_legacy_audit: ProbeRequestAudit =
            serde_json::from_value(legacy_audit).unwrap();
        assert_eq!(restored_legacy_audit.max_output_tokens, 0);
        assert_eq!(request.audit.surface_timeline.len(), 4);
        assert_eq!(
            request
                .slots
                .values()
                .filter(|slot| slot.category == SupportCategory::ContextImage)
                .count(),
            3
        );
        assert!(request
            .slots
            .values()
            .any(|slot| slot.category == SupportCategory::ImageAfter));
        assert!(!request
            .slots
            .values()
            .any(|slot| slot.category == SupportCategory::ImageBefore));

        let structured_text = request
            .body
            .pointer("/input/1/content/0/text")
            .and_then(Value::as_str)
            .expect("structured request text");
        let structured: Value = serde_json::from_str(structured_text).unwrap();
        let timeline = structured
            .get("recent_surface_timeline")
            .and_then(Value::as_array)
            .expect("surface timeline");
        assert_eq!(timeline.len(), 4);
        assert_eq!(
            timeline[0].get("app_label").and_then(Value::as_str),
            Some("ChatGPT")
        );
        assert_eq!(
            timeline[1].get("site_hostname").and_then(Value::as_str),
            Some("devfolio.co")
        );
        assert_eq!(
            timeline[2].get("site_hostname").and_then(Value::as_str),
            Some("google.com")
        );
        assert_eq!(
            timeline[3].get("site_hostname").and_then(Value::as_str),
            Some("platform.openai.com")
        );
        assert!(!structured_text.contains("/logs/resp_"));
        assert!(!structured_text.contains("?q="));
        assert!(!structured_text.contains("page_title"));

        let primary_slots = request
            .body
            .pointer("/text/format/schema/properties/support_slots_by_field/properties/primary_task/items/enum")
            .and_then(Value::as_array)
            .expect("primary task support slots");
        assert!(primary_slots.iter().filter_map(Value::as_str).any(|slot| {
            request
                .slots
                .get(slot)
                .is_some_and(|support| support.category == SupportCategory::ContextImage)
        }));
    }

    #[test]
    fn incomplete_provider_response_is_rejected_as_a_transport_failure() {
        let error = output_text(&json!({
            "status": "incomplete",
            "incomplete_details": {"reason": "max_output_tokens"},
            "output_text": "{\"primary_task\":\"cut off"
        }))
        .expect_err("an incomplete envelope must never be parsed as task truth");

        assert_eq!(error.0, ProbeDiagnosticStatus::ProviderNoUsableOutput);
        assert_eq!(error.1, "provider_response_incomplete");
    }

    #[test]
    fn timeline_metadata_is_not_a_semantic_support_slot_and_private_visits_are_redacted() {
        let mut packet = live_shaped_session_packet();
        packet.surface_timeline[1].private = true;
        packet.surface_timeline[1].app_label = "Sensitive Private App".into();
        packet.surface_timeline[1].site_hostname = Some("secret.example".into());
        packet.surface_timeline[1].representative_frame = None;
        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("request");
        let structured_text = request
            .body
            .pointer("/input/1/content/0/text")
            .and_then(Value::as_str)
            .expect("structured request text");
        assert!(!structured_text.contains("Sensitive Private App"));
        assert!(!structured_text.contains("secret.example"));
        let private_audit = request
            .audit
            .surface_timeline
            .iter()
            .find(|visit| visit.private)
            .expect("private audit visit");
        assert_eq!(private_audit.app_label, "Private activity");
        assert!(private_audit.site_hostname.is_none());
        assert_eq!(
            private_audit.image_omission_reason.as_deref(),
            Some("private")
        );
        assert_eq!(
            request.audit.image_exclusions_by_reason.get("private"),
            Some(&1)
        );

        let proposed = ProbeModelOutput {
            primary_task: Some("A task guessed only from the app timeline".into()),
            current_step: None,
            last_progress: None,
            unfinished_state: None,
            visit_roles: BTreeMap::new(),
            support_slots_by_field: BTreeMap::from([
                ("primary_task".into(), Vec::new()),
                ("current_step".into(), Vec::new()),
                ("last_progress".into(), Vec::new()),
                ("unfinished_state".into(), Vec::new()),
            ]),
            missing_evidence: Vec::new(),
            confidence_by_field: BTreeMap::new(),
            status: ProbeResolutionStatus::PartlyResolved,
        };
        let (admitted, issues) = admit_output(&packet, &request, proposed);
        assert!(admitted.primary_task.is_none());
        assert!(issues
            .iter()
            .any(|issue| { issue.contains("primary_task") && issue.contains("support") }));
    }

    #[test]
    fn image_budget_omissions_are_typed_per_surface_visit() {
        let mut packet = live_shaped_session_packet();
        let mut extra = keyframe(2, false);
        extra.frame_id = "frame-extra".into();
        extra.observed_at_ms = 2_500;
        extra.local_image_handle_hash = Some("image-hash-extra".into());
        extra.ephemeral_local_image_path = Some(image_file(55));
        packet.surface_timeline.insert(
            3,
            surface_visit(4, "Helium", Some("github.com"), extra, 1, false),
        );
        packet.surface_timeline[4].sequence_index = 5;

        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("request");
        let omitted = request
            .audit
            .surface_timeline
            .iter()
            .find(|visit| visit.site_hostname.as_deref() == Some("github.com"))
            .expect("omitted surface audit");
        assert!(omitted.image_slot.is_none());
        assert_eq!(
            omitted.image_omission_reason.as_deref(),
            Some("budget_omitted")
        );
        assert_eq!(request.audit.omitted_surface_visit_count, 1);
        assert_eq!(
            request
                .audit
                .image_exclusions_by_reason
                .get("budget_omitted"),
            Some(&1)
        );
    }

    #[test]
    fn supplied_log_style_cutoff_excludes_all_eleven_late_events() {
        let mut packet = packet(false);
        for index in 0..11 {
            let mut late = event(4);
            late.event_id = format!("late-event-{index}");
            late.observed_at_ms = 4_001 + index;
            packet.causal_events.push(late);
        }
        let mut late_observation = element(4);
        late_observation.element_id = "late-observation".into();
        late_observation.changed_at_ms = 4_050;
        packet.canonical_elements.push(late_observation);
        let mut future_frame = keyframe(5, false);
        future_frame.observed_at_ms = 4_100;
        future_frame.frame_id = "future-frame".into();
        packet.semantic_keyframes.push(future_frame);
        let mut late_delta = delta(4);
        late_delta.delta_id = "late-delta".into();
        late_delta.next_frame_id = "future-frame".into();
        packet.frame_changes.push(late_delta);

        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("build request");
        assert_eq!(request.audit.final_frame_id, "frame-4");
        assert_eq!(request.audit.cutoff_observed_at_ms, 4_000);
        assert_eq!(
            request
                .audit
                .excluded_late_records_by_kind
                .get("causal_event"),
            Some(&11)
        );
        assert_eq!(
            request
                .audit
                .excluded_late_records_by_kind
                .get("canonical_observation"),
            Some(&1)
        );
        assert_eq!(
            request
                .audit
                .excluded_late_records_by_kind
                .get("semantic_delta"),
            Some(&1)
        );
        assert!(request
            .slots
            .values()
            .all(|slot| slot.observed_at_ms <= request.audit.cutoff_observed_at_ms));
        assert!(!request
            .slots
            .values()
            .any(|slot| slot.record_id.starts_with("late-") || slot.record_id == "future-frame"));
        assert!(request.audit.structured_bytes < 14_046);
        eprintln!(
            "compact_cutoff_fixture structured_bytes={} estimated_tokens={} images={} boundaries={}",
            request.audit.structured_bytes,
            request.audit.estimated_text_tokens,
            request.audit.image_count,
            request.audit.boundary_count
        );
    }

    #[test]
    fn future_target_frame_cannot_pull_an_earlier_event_into_the_request() {
        let mut packet = packet(false);
        let mut future_frame = keyframe(5, false);
        future_frame.observed_at_ms = 4_500;
        future_frame.frame_id = "future-target".into();
        packet.semantic_keyframes.push(future_frame);
        let mut event = event(4);
        event.event_id = "early-event-with-future-target".into();
        event.observed_at_ms = 3_950;
        event.target_frame_id = Some("future-target".into());
        packet.causal_events.push(event);

        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("build request");
        assert!(!request
            .slots
            .values()
            .any(|slot| slot.record_id == "early-event-with-future-target"));
        assert_eq!(
            request
                .audit
                .excluded_late_records_by_kind
                .get("causal_event"),
            Some(&1)
        );
    }

    #[test]
    fn unrepresented_target_frame_cannot_be_assumed_to_precede_the_cutoff() {
        let mut packet = packet(false);
        let mut event = event(4);
        event.event_id = "event-with-unrepresented-target".into();
        event.observed_at_ms = 3_950;
        event.target_frame_id = Some("frame-not-in-packet".into());
        packet.causal_events.push(event);

        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("build request");
        assert!(!request
            .slots
            .values()
            .any(|slot| slot.record_id == "event-with-unrepresented-target"));
    }

    #[test]
    fn passive_accessibility_notifications_never_become_user_action_slots() {
        let mut packet = packet(false);
        packet.causal_events.clear();
        for index in 0..6 {
            let mut notification = event(4);
            notification.event_id = format!("notification-{index}");
            notification.event_kind = "accessibility_focus_notification".into();
            notification.source = "accessibility".into();
            notification.observed_at_ms = 3_800 + index;
            packet.causal_events.push(notification);
        }
        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("build request");
        assert!(!request
            .slots
            .values()
            .any(|slot| slot.category == SupportCategory::UserAction));
        assert_eq!(
            request
                .audit
                .excluded_nonsemantic_records_by_kind
                .get("passive_or_no_effect_event"),
            Some(&6)
        );
    }

    #[test]
    fn duplicates_and_no_effect_scrolls_are_collapsed_before_serialization() {
        let mut packet = packet(false);
        let mut duplicate = packet.canonical_elements[3].clone();
        duplicate.element_id = "duplicate-element".into();
        duplicate.source_conflicts = vec![
            "ax_ocr_text_disagreement".into(),
            "ax_ocr_text_disagreement".into(),
        ];
        duplicate.rejection_reasons = vec!["repeated_reason".into(), "repeated_reason".into()];
        packet.canonical_elements[3].source_conflicts = vec!["ax_ocr_text_disagreement".into()];
        packet.canonical_elements.push(duplicate);
        let mut no_op_scroll = event(4);
        no_op_scroll.event_id = "duplicate-no-op-scroll".into();
        no_op_scroll.event_kind = "scroll".into();
        no_op_scroll.semantic_delta_reference = Some("no-op-delta".into());
        packet.causal_events.push(no_op_scroll);
        let mut no_op_delta = delta(4);
        no_op_delta.delta_id = "no-op-delta".into();
        no_op_delta.no_observable_change = true;
        no_op_delta.observable_changes.clear();
        packet.frame_changes.push(no_op_delta);

        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("build request");
        assert!(request
            .audit
            .deduplication_counts
            .get("canonical_observation")
            .is_some_and(|count| *count >= 1));
        assert!(request
            .audit
            .deduplication_counts
            .get("reason_string")
            .is_some_and(|count| *count >= 2));
        assert_eq!(
            request
                .slots
                .values()
                .filter(|slot| slot.content_hash.as_deref() == Some("text-hash-4"))
                .count(),
            1
        );
        assert!(!request.slots.values().any(|slot| {
            slot.record_id == "duplicate-no-op-scroll" || slot.record_id == "no-op-delta"
        }));
    }

    #[test]
    fn current_boundary_preserves_distinct_before_after_and_delta_facts() {
        let request = build_probe_request(&packet(false), DEFAULT_LUNA_MODEL).expect("request");
        assert!(request.slots.contains_key("B1_IMAGE_BEFORE"));
        assert!(request.slots.contains_key("B1_IMAGE_AFTER"));
        assert!(request
            .slots
            .values()
            .any(|slot| slot.category == SupportCategory::Delta));
        assert_eq!(
            request.audit.boundary_selection_reasons,
            vec!["current_manual_boundary"]
        );
    }

    #[test]
    fn live_like_devfolio_to_x_scroll_keeps_context_and_sends_readable_evidence() {
        let mut packet = packet(false);
        let browser_surface = |page: &str| ActiveSurfaceIdentityV2 {
            app_name: Some("Helium".into()),
            app_bundle_id: Some("net.imput.helium".into()),
            window_title_hash: Some(format!("title-{page}")),
            window_id: None,
            browser_url_hash: Some(format!("url-{page}")),
            document_path_hash: None,
        };
        for (index, frame) in packet.semantic_keyframes.iter_mut().enumerate() {
            frame.surface_identity = match index {
                0 => browser_surface("wemakedevs"),
                1 => browser_surface("devfolio-application"),
                2 | 3 => browser_surface("x-home"),
                _ => unreachable!(),
            };
            frame.partition = if index == 3 {
                EvidencePartitionV2::Current
            } else {
                EvidencePartitionV2::Prior
            };
        }
        packet.current_frame = packet.semantic_keyframes[3].clone();
        packet.active_surface = packet.current_frame.surface_identity.clone();
        packet.canonical_elements.clear();
        packet.focused_element_ids.clear();
        packet.editable_element_ids.clear();

        let mut scroll = event(4);
        scroll.event_id = "x-scroll".into();
        scroll.event_kind = "scroll".into();
        scroll.observed_at_ms = 3_900;
        scroll.frame_id = "frame-4".into();
        scroll.source_frame_id = "frame-3".into();
        scroll.target_frame_id = Some("frame-4".into());
        scroll.target_element_id = None;
        scroll.app_bundle_id = Some("net.imput.helium".into());
        scroll.semantic_delta_reference = Some("delta-4".into());
        scroll.committed = None;
        packet.causal_events = vec![scroll];

        let mut entered_x = delta(3);
        entered_x.prior_frame_id = Some("frame-2".into());
        entered_x.next_frame_id = "frame-3".into();
        entered_x.frame_id = "frame-3".into();
        entered_x.diff_kind = Some("unknown".into());
        entered_x.observable_changes = vec!["content_appeared".into()];
        entered_x.causal_event_ids.clear();
        let mut scrolled_x = delta(4);
        scrolled_x.prior_frame_id = Some("frame-3".into());
        scrolled_x.next_frame_id = "frame-4".into();
        scrolled_x.frame_id = "frame-4".into();
        scrolled_x.observable_changes = vec!["content_appeared".into()];
        scrolled_x.causal_event_ids = vec!["x-scroll".into()];
        packet.frame_changes = vec![entered_x, scrolled_x];

        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("compact request");

        assert_eq!(request.audit.boundary_count, 2);
        assert_eq!(request.audit.image_count, 3);
        assert_eq!(
            request.audit.boundary_selection_reasons,
            vec![
                "recent_surface_transition_with_activity",
                "current_manual_boundary"
            ]
        );
        assert_eq!(
            request
                .slots
                .values()
                .filter(|slot| matches!(
                    slot.category,
                    SupportCategory::ImageBefore | SupportCategory::ImageAfter
                ))
                .map(|slot| slot.record_id.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["frame-2", "frame-3", "frame-4"])
        );
        let record_ids = request
            .slots
            .values()
            .map(|slot| slot.record_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            record_ids.len(),
            record_ids.iter().copied().collect::<BTreeSet<_>>().len()
        );

        let structured_text = request
            .body
            .pointer("/input/1/content/0/text")
            .and_then(Value::as_str)
            .expect("structured request text");
        assert!(structured_text.contains("earlier_context"));
        assert!(structured_text.contains("current_at_continue"));
        assert!(structured_text.contains("different page, document, or window"));
        assert!(structured_text.contains("The user scrolled on this surface"));
        assert!(!structured_text.contains("change_kind"));
        assert!(!structured_text.contains("observable_changes"));
        assert!(!structured_text.contains("committed=None"));
        assert!(!structured_text.contains("transition:continuing_same_task"));

        let earlier_image = request
            .slots
            .values()
            .find(|slot| {
                slot.boundary_index == 1
                    && matches!(
                        slot.category,
                        SupportCategory::ImageBefore | SupportCategory::ImageAfter
                    )
            })
            .unwrap()
            .slot
            .clone();
        let proposed = ProbeModelOutput {
            primary_task: Some("Complete the visible hackathon application".into()),
            current_step: None,
            last_progress: None,
            unfinished_state: None,
            visit_roles: visit_roles(&request),
            support_slots_by_field: BTreeMap::from([
                ("primary_task".into(), vec![earlier_image]),
                ("current_step".into(), Vec::new()),
                ("last_progress".into(), Vec::new()),
                ("unfinished_state".into(), Vec::new()),
            ]),
            missing_evidence: Vec::new(),
            confidence_by_field: SEMANTIC_FIELDS
                .iter()
                .map(|field| ((*field).into(), 0.8))
                .collect(),
            status: ProbeResolutionStatus::PartlyResolved,
        };
        let (admitted, issues) = admit_output(&packet, &request, proposed);
        assert_eq!(
            admitted.primary_task.as_deref(),
            Some("Complete the visible hackathon application")
        );
        assert!(!issues
            .iter()
            .any(|issue| issue.contains("passive_evidence_cannot_establish_primary_task")));
    }

    #[test]
    fn thin_current_boundary_adds_one_grounded_earlier_boundary() {
        let mut packet = packet(false);
        let current_event = packet
            .causal_events
            .iter_mut()
            .find(|event| event.event_id == "event-4")
            .expect("current event");
        current_event.event_kind = "accessibility_focus_notification".into();
        current_event.source = "accessibility".into();
        current_event.committed = None;

        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("request");
        assert_eq!(request.audit.boundary_count, 2);
        assert_eq!(request.audit.image_count, 3);
        assert_eq!(
            request.audit.boundary_selection_reasons,
            vec!["committed_action_with_result", "current_manual_boundary"]
        );
        assert!(request
            .slots
            .values()
            .filter(|slot| slot.boundary_index == 1)
            .any(|slot| slot.category == SupportCategory::Delta));
        assert!(!request
            .slots
            .values()
            .filter(|slot| slot.boundary_index == 2)
            .any(|slot| slot.category == SupportCategory::UserAction));
    }

    #[test]
    fn serialized_boundary_slots_are_chronological() {
        let mut packet = packet(false);
        let current_event = packet
            .causal_events
            .iter_mut()
            .find(|event| event.event_id == "event-4")
            .expect("current event");
        current_event.event_kind = "accessibility_focus_notification".into();
        current_event.source = "accessibility".into();
        current_event.committed = None;
        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("request");
        let structured_text = request
            .body
            .pointer("/input/1/content/0/text")
            .and_then(Value::as_str)
            .expect("structured request text");
        let structured: Value = serde_json::from_str(structured_text).expect("structured JSON");
        for boundary in structured
            .get("boundaries")
            .and_then(Value::as_array)
            .expect("boundaries")
        {
            let timestamps = boundary
                .get("slots")
                .and_then(Value::as_array)
                .expect("slots")
                .iter()
                .map(|slot| {
                    slot.get("observed_at_ms")
                        .and_then(Value::as_i64)
                        .expect("slot timestamp")
                })
                .collect::<Vec<_>>();
            assert!(timestamps.windows(2).all(|pair| pair[0] <= pair[1]));
        }
    }

    #[test]
    fn unrelated_recent_frame_is_not_selected_as_prior_context() {
        let mut packet = packet(false);
        let mut unrelated = keyframe(9, false);
        unrelated.frame_id = "unrelated-recent-frame".into();
        unrelated.observed_at_ms = 3_500;
        unrelated.surface_identity.app_bundle_id = Some("com.example.unrelated".into());
        unrelated.surface_identity.document_path_hash = Some("unrelated-document".into());
        unrelated.surface_identity.window_title_hash = Some("unrelated-window".into());
        unrelated.surface_identity.window_id = Some(999);
        packet.semantic_keyframes.push(unrelated);

        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("request");
        assert!(!request
            .slots
            .values()
            .any(|slot| slot.record_id == "unrelated-recent-frame"));
        assert_eq!(request.audit.boundary_count, 1);
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
    fn boundary_image_and_per_category_caps_fail_closed() {
        let request = build_probe_request(&packet(false), DEFAULT_LUNA_MODEL).expect("request");
        assert!(!request_category_caps_allowed(
            MAX_BOUNDARIES + 1,
            request.audit.image_count,
            &request.slots
        ));
        assert!(!request_category_caps_allowed(
            request.audit.boundary_count,
            MAX_IMAGES + 1,
            &request.slots
        ));
        let mut overfull_slots = request.slots.clone();
        let template = overfull_slots
            .values()
            .find(|slot| slot.category == SupportCategory::OwnedObservation)
            .expect("owned observation")
            .clone();
        for index in 0..=MAX_OBSERVATIONS_PER_BOUNDARY {
            let mut extra = template.clone();
            extra.slot = format!("B{}_EXTRA_OBSERVATION_{index}", template.boundary_index);
            overfull_slots.insert(extra.slot.clone(), extra);
        }
        assert!(!request_category_caps_allowed(
            request.audit.boundary_count,
            request.audit.image_count,
            &overfull_slots
        ));
    }

    #[test]
    fn private_current_boundary_is_blocked_before_transport() {
        let error = build_probe_request(&packet(true), "model-a").unwrap_err();
        assert_eq!(error.0, ProbeDiagnosticStatus::PrivacyBlocked);
    }

    #[test]
    fn stale_missing_and_model_ineligible_current_frames_are_typed_failures() {
        let mut stale = packet(false);
        stale.observed_at_ms = stale.current_frame.observed_at_ms + 1;
        let error = build_probe_request(&stale, DEFAULT_LUNA_MODEL).unwrap_err();
        assert_eq!(error.0, ProbeDiagnosticStatus::RequestNotBuilt);
        assert_eq!(error.1, "current_frame_stale");

        let mut missing = packet(false);
        missing.current_frame.frame_id.clear();
        let error = build_probe_request(&missing, DEFAULT_LUNA_MODEL).unwrap_err();
        assert_eq!(error.1, "current_frame_missing");

        let mut ineligible = packet(false);
        ineligible.current_frame.model_eligible = false;
        let error = build_probe_request(&ineligible, DEFAULT_LUNA_MODEL).unwrap_err();
        assert_eq!(error.1, "current_frame_model_ineligible");
    }

    #[test]
    fn runtime_probe_rejects_every_model_except_luna() {
        assert_eq!(configured_model_name(), DEFAULT_LUNA_MODEL);
        assert!(public_authority_enabled());
        assert_eq!(MANUAL_PROVIDER_RETRIES, 0);
        let (attempt, slots) = run_probe(&packet(false), "gpt-5.6-sol", None);
        assert_eq!(
            attempt.diagnostic_status,
            ProbeDiagnosticStatus::RequestNotBuilt
        );
        assert_eq!(
            attempt.failure_reason.as_deref(),
            Some("semantic_probe_requires_gpt_5_6_luna")
        );
        assert!(slots.is_none());
    }

    #[test]
    fn production_runtime_case_needs_no_armed_input_and_one_decision_is_idempotent() {
        let conn = Connection::open_in_memory().expect("open database");
        ensure_schema(&conn).expect("semantic probe schema");
        let decision_id = "decision-production-compact";
        let now_ms = super::super::super::current_time_millis();
        let case_id = ensure_production_runtime_case(&conn, decision_id, now_ms)
            .expect("create internal production case");
        let case_kind: String = conn
            .query_row(
                "SELECT case_kind FROM task_truth_v2_semantic_probe_cases WHERE case_id=?1",
                [case_id.as_str()],
                |row| row.get(0),
            )
            .expect("read production case");
        assert_eq!(case_kind, "production_runtime");

        let attempt = ProbeAttempt {
            diagnostic_status: ProbeDiagnosticStatus::RequestNotBuilt,
            model: DEFAULT_LUNA_MODEL.into(),
            request_id: None,
            provider_request_id: None,
            response_id: None,
            response_model: None,
            request_audit: None,
            usage: ProviderUsageV1::default(),
            estimated_cost_usd: None,
            latency_ms: 0,
            output_bytes: None,
            parsed_response: false,
            provider_post_count: 0,
            cited_support_slots_before_admission: BTreeMap::new(),
            admitted_output: None,
            validation_issues: Vec::new(),
            failure_reason: Some("test_preflight_failure".into()),
        };
        let packet = packet(false);
        persist_attempt(
            &conn,
            &case_id,
            decision_id,
            packet.session_id.as_deref(),
            &packet,
            &attempt,
            None,
        )
        .expect("persist compact run");

        // This returns before loading any armed case or provider config. The
        // exact decision already owns a compact run and cannot submit again.
        run_manual_probe(
            &conn,
            decision_id,
            packet.session_id.as_deref(),
            &packet,
            None,
        )
        .expect("reuse exact decision");
        let run_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_truth_v2_semantic_probe_runs WHERE decision_id=?1",
                [decision_id],
                |row| row.get(0),
            )
            .expect("count compact runs");
        assert_eq!(run_count, 1);
    }

    #[test]
    fn compact_only_mode_cannot_fall_back_to_the_legacy_request_path() {
        assert!(probe_mode_enabled(Some("1".into()), None));
        assert!(probe_mode_enabled(Some("0".into()), Some("true".into())));
        assert!(!probe_mode_enabled(Some("0".into()), Some("off".into())));
    }

    #[test]
    fn foreign_slot_nulls_only_that_field_without_semantic_replacement() {
        let packet = packet(false);
        let request = build_probe_request(&packet, "model-a").expect("build request");
        let mut output = output(Some("Implement the PFTU semantic probe"), &request);
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
        let generated = output(Some("Implement the PFTU semantic probe"), &request);
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

        let mut generated = output(Some("Implement the PFTU semantic probe"), &request);
        generated
            .support_slots_by_field
            .insert("primary_task".into(), vec!["B1_IMAGE_BEFORE".into()]);
        let (admitted, issues) = admit_output(&packet, &request, generated);
        assert!(admitted.primary_task.is_none());
        assert!(issues.iter().any(|issue| {
            issue == "primary_task:passive_evidence_cannot_establish_primary_task"
        }));
    }

    #[test]
    fn cited_slots_are_preserved_before_field_level_admission() {
        let packet = packet(false);
        let mut request = build_probe_request(&packet, "model-a").expect("build request");
        request
            .slots
            .get_mut("B1_IMAGE_BEFORE")
            .expect("image slot")
            .category = SupportCategory::SurfaceIdentity;
        let mut generated = output(Some("Implement the PFTU semantic probe"), &request);
        generated
            .support_slots_by_field
            .insert("current_step".into(), vec!["B1_IMAGE_BEFORE".into()]);
        let response = json!({"output_text": serde_json::to_string(&generated).unwrap()});
        let (admitted, _, issues, cited_before_admission) =
            parse_probe_response(&packet, &request, &response).expect("parse response");
        assert_eq!(admitted.current_step, None);
        assert_eq!(
            cited_before_admission.get("current_step"),
            Some(&vec!["B1_IMAGE_BEFORE".into()])
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
            output(Some("Implement the PFTU semantic probe"), &request),
        );
        assert_eq!(admitted.unfinished_state, None);
        assert!(issues
            .iter()
            .any(|issue| issue == "unfinished_state:stale_or_ineligible_slot"));
    }

    #[test]
    fn context_image_uses_physical_identity_not_selection_metadata() {
        let mut packet = live_shaped_session_packet();
        packet.semantic_keyframes[0].partition = EvidencePartitionV2::Background;
        packet.semantic_keyframes[0].selection_reasons = vec!["semantic_keyframe_reason".into()];
        packet.surface_timeline[0]
            .representative_frame
            .as_mut()
            .expect("timeline representative")
            .selection_reasons = vec!["session_surface_representative".into()];
        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("request");
        let context_slot = request
            .slots
            .values()
            .find(|slot| slot.category == SupportCategory::ContextImage)
            .expect("context image")
            .slot
            .clone();
        let mut generated = output(Some("Audit the visible continuation contract"), &request);
        generated
            .support_slots_by_field
            .insert("primary_task".into(), vec![context_slot]);

        let (admitted, issues) = admit_output(&packet, &request, generated);
        assert_eq!(
            admitted.primary_task.as_deref(),
            Some("Audit the visible continuation contract")
        );
        assert!(!issues
            .iter()
            .any(|issue| issue == "primary_task:stale_or_ineligible_slot"));
    }

    #[test]
    fn conflicting_physical_variants_still_reject_the_context_slot() {
        let mut packet = live_shaped_session_packet();
        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("request");
        let context_slot = request
            .slots
            .values()
            .find(|slot| slot.category == SupportCategory::ContextImage)
            .expect("context image")
            .slot
            .clone();
        packet.semantic_keyframes[0].observed_at_ms += 1;
        let mut generated = output(Some("Audit the visible continuation contract"), &request);
        generated
            .support_slots_by_field
            .insert("primary_task".into(), vec![context_slot]);

        let (admitted, issues) = admit_output(&packet, &request, generated);
        assert!(admitted.primary_task.is_none());
        assert!(issues
            .iter()
            .any(|issue| issue == "primary_task:stale_or_ineligible_slot"));
    }

    #[test]
    fn visit_role_schema_is_exact_and_bad_role_citations_are_field_local() {
        let packet = live_shaped_session_packet();
        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("request");
        let schema_roles = request
            .body
            .pointer("/text/format/schema/properties/visit_roles/properties")
            .and_then(Value::as_object)
            .expect("visit role properties");
        let required_roles = request
            .body
            .pointer("/text/format/schema/properties/visit_roles/required")
            .and_then(Value::as_array)
            .expect("required visit roles");
        assert_eq!(schema_roles.len(), request.audit.image_count);
        assert_eq!(required_roles.len(), request.audit.image_count);
        assert!(schema_roles.contains_key("T1_VISIT"));
        assert!(schema_roles.contains_key("T4_VISIT"));

        let mut generated = output(Some("Audit the visible continuation contract"), &request);
        let wrong_image = request
            .audit
            .surface_timeline
            .iter()
            .find(|visit| visit.visit_id == "T2_VISIT")
            .and_then(|visit| visit.image_slot.clone())
            .expect("second visit image");
        generated
            .visit_roles
            .get_mut("T1_VISIT")
            .expect("first visit role")
            .support_slots = vec![wrong_image];
        generated
            .visit_roles
            .get_mut("T2_VISIT")
            .expect("second visit role")
            .role = ProbeSurfaceRole::SupportingWork;

        let (admitted, issues) = admit_output(&packet, &request, generated);
        assert_eq!(
            admitted.visit_roles["T1_VISIT"].role,
            ProbeSurfaceRole::Unclear
        );
        assert_eq!(admitted.visit_roles["T1_VISIT"].confidence, 0.0);
        assert_eq!(
            admitted.visit_roles["T2_VISIT"].role,
            ProbeSurfaceRole::SupportingWork
        );
        assert_eq!(
            admitted.primary_task.as_deref(),
            Some("Audit the visible continuation contract")
        );
        assert!(issues
            .iter()
            .any(|issue| issue == "visit_role:T1_VISIT:missing_own_visit_image_slot"));
    }

    #[test]
    fn old_probe_output_without_visit_roles_still_deserializes() {
        let mut legacy = serde_json::to_value(output(
            Some("Implement the PFTU semantic probe"),
            &build_probe_request(&packet(false), DEFAULT_LUNA_MODEL).expect("request"),
        ))
        .unwrap();
        legacy
            .as_object_mut()
            .expect("probe output")
            .remove("visit_roles");
        let restored: ProbeModelOutput = serde_json::from_value(legacy).unwrap();
        assert!(restored.visit_roles.is_empty());
    }

    #[test]
    fn slot_round_trip_rejects_chronology_drift_even_when_source_hash_matches() {
        let packet = packet(false);
        let mut request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("request");
        let action_slot = latest_slot(&request, SupportCategory::UserAction);
        request
            .slots
            .get_mut(&action_slot)
            .expect("user action")
            .observed_at_ms = request.audit.cutoff_observed_at_ms + 1;
        let (admitted, issues) = admit_output(
            &packet,
            &request,
            output(Some("Implement the PFTU semantic probe"), &request),
        );
        assert_eq!(admitted.current_step, None);
        assert!(issues
            .iter()
            .any(|issue| issue == "current_step:slot_chronology_invalid"));
    }

    #[test]
    fn privacy_blocked_slot_is_rejected_and_only_its_field_is_nulled() {
        let packet = packet(false);
        let mut request = build_probe_request(&packet, "model-a").expect("build request");
        let observation_slot = latest_slot(&request, SupportCategory::OwnedObservation);
        request
            .slots
            .get_mut(&observation_slot)
            .expect("owned observation slot")
            .privacy_eligible = false;
        let (admitted, issues) = admit_output(
            &packet,
            &request,
            output(Some("Implement the PFTU semantic probe"), &request),
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
        let (admitted, issues) =
            admit_output(&packet, &request, output(Some("Editing code"), &request));
        assert_eq!(admitted.primary_task, None);
        assert!(issues
            .iter()
            .any(|issue| issue == "primary_task:forbidden_generic_primary_task"));
    }

    #[test]
    fn concrete_purpose_with_activity_prefix_is_preserved() {
        let packet = packet(false);
        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("request");
        let purpose = "Reviewing the agent output for future-event leakage";
        let (admitted, issues) = admit_output(&packet, &request, output(Some(purpose), &request));
        assert!(issues.is_empty(), "{issues:?}");
        assert_eq!(admitted.primary_task.as_deref(), Some(purpose));
    }

    #[test]
    fn overlong_semantic_field_is_nulled_instead_of_truncated_or_rewritten() {
        let packet = packet(false);
        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("request");
        let overlong = "x".repeat(MAX_SEMANTIC_FIELD_CHARS + 1);
        let mut generated = output(Some("Implement the PFTU semantic probe"), &request);
        generated.current_step = Some(overlong);
        let (admitted, issues) = admit_output(&packet, &request, generated);
        assert_eq!(admitted.current_step, None);
        assert!(issues
            .iter()
            .any(|issue| issue == "current_step:field_value_too_long"));
    }

    #[test]
    fn passive_scrolling_without_visible_objective_admits_no_primary_task() {
        let mut packet = packet(false);
        for element in &mut packet.canonical_elements {
            element.authorship_status = AuthorshipStatusV2::ApplicationOrAgent;
            element.causal_evidence_refs.clear();
        }
        packet.causal_events.clear();
        let mut scroll = event(4);
        scroll.event_id = "passive-scroll".into();
        scroll.event_kind = "scroll".into();
        scroll.committed = None;
        scroll.semantic_delta_reference = Some("delta-4".into());
        packet.causal_events.push(scroll);
        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("request");
        let primary_slot = request
            .slots
            .values()
            .find(|slot| {
                matches!(
                    slot.category,
                    SupportCategory::ImageBefore | SupportCategory::ImageAfter
                )
            })
            .expect("image support")
            .slot
            .clone();
        let mut generated = ProbeModelOutput {
            primary_task: Some("Research the product described on the page".into()),
            current_step: None,
            last_progress: None,
            unfinished_state: None,
            visit_roles: BTreeMap::new(),
            support_slots_by_field: SEMANTIC_FIELDS
                .iter()
                .map(|field| ((*field).into(), Vec::new()))
                .collect(),
            missing_evidence: Vec::new(),
            confidence_by_field: SEMANTIC_FIELDS
                .iter()
                .map(|field| ((*field).into(), 0.0))
                .collect(),
            status: ProbeResolutionStatus::PartlyResolved,
        };
        generated
            .support_slots_by_field
            .insert("primary_task".into(), vec![primary_slot]);
        generated
            .confidence_by_field
            .insert("primary_task".into(), 0.9);
        let (admitted, issues) = admit_output(&packet, &request, generated);
        assert_eq!(admitted.primary_task, None);
        assert_eq!(admitted.status, ProbeResolutionStatus::Unresolved);
        assert!(issues.iter().any(|issue| {
            issue == "primary_task:passive_evidence_cannot_establish_primary_task"
        }));
    }

    #[test]
    fn unrelated_action_cannot_justify_a_primary_task_cited_only_to_a_passive_image() {
        let packet = packet(false);
        let request = build_probe_request(&packet, DEFAULT_LUNA_MODEL).expect("request");
        assert!(request
            .slots
            .values()
            .any(|slot| slot.category == SupportCategory::UserAction));
        let passive_image = latest_slot(&request, SupportCategory::ImageAfter);
        let mut generated = output(Some("Infer a purpose from the final screen"), &request);
        generated
            .support_slots_by_field
            .insert("primary_task".into(), vec![passive_image]);

        let (admitted, issues) = admit_output(&packet, &request, generated);
        assert!(admitted.primary_task.is_none());
        assert!(issues.iter().any(|issue| {
            issue == "primary_task:passive_evidence_cannot_establish_primary_task"
        }));
    }

    #[test]
    fn provider_payload_contains_only_short_slots_and_neutral_boundary_metadata() {
        let request = build_probe_request(&packet(false), DEFAULT_LUNA_MODEL).expect("request");
        let structured_text = request
            .body
            .pointer("/input/1/content/0/text")
            .and_then(Value::as_str)
            .expect("structured request text");
        assert!(!structured_text.contains("packet-probe"));
        assert!(!structured_text.contains("com.example.editor"));
        assert!(!structured_text.contains("document-hash-semantic-probe"));
        assert!(!structured_text.contains("SURFACE"));
        assert!(structured_text.contains("current_manual_boundary"));
        assert!(request.slots.keys().all(|slot| slot.starts_with('B')));
    }

    #[test]
    fn parsing_distinguishes_resolved_partial_unresolved_refusal_empty_and_malformed_output() {
        let packet = packet(false);
        let request = build_probe_request(&packet, "model-a").expect("build request");
        let resolved = json!({"output_text":serde_json::to_string(&output(Some("Implement the PFTU semantic probe"), &request)).unwrap()});
        assert_eq!(
            parse_probe_response(&packet, &request, &resolved)
                .expect("resolved response")
                .0
                .status,
            ProbeResolutionStatus::Resolved
        );

        let mut partial_output = output(Some("Implement the PFTU semantic probe"), &request);
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

        let mut unresolved_output = output(None, &request);
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
        let (attempt, _) = run_probe(&packet(false), DEFAULT_LUNA_MODEL, None);
        assert_eq!(
            attempt.diagnostic_status,
            ProbeDiagnosticStatus::ProviderUnavailable
        );
        assert!(!attempt.parsed_response);
        assert_eq!(attempt.provider_post_count, 0);
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
