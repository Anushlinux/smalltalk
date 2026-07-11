use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    stable_hash, task_turn_evidence::is_categorical_control_hint, EvidenceContentUnit,
    EvidenceFrame, EvidenceOcrSpan, Rect,
};

pub(crate) const OBSERVATION_PACKET_SCHEMA_V2: &str = "smalltalk.observation_packet.v2";
const MAX_KEYFRAMES: usize = 4;
const MAX_ELEMENTS: usize = 160;
const MAX_CAUSAL_EVENTS: usize = 96;
const MAX_NOTES: usize = 32;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvidencePartitionV2 {
    Current,
    Prior,
    Background,
    Support,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RegionRoleV2 {
    PrimaryContent,
    UserAuthoredContent,
    ApplicationAgentOutput,
    ComposerEditor,
    Navigation,
    Toolbar,
    Control,
    Status,
    Notification,
    Sidebar,
    Modal,
    BrowserChrome,
    TerminalInput,
    TerminalOutput,
    DocumentCanvas,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuthorshipStatusV2 {
    User,
    ApplicationOrAgent,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct PacketBoundsV2 {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

impl From<Rect> for PacketBoundsV2 {
    fn from(value: Rect) -> Self {
        Self {
            x: value.x,
            y: value.y,
            width: value.w,
            height: value.h,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct EvidenceHandleV2 {
    pub(crate) source_kind: String,
    pub(crate) record_id: String,
    pub(crate) frame_id: Option<String>,
    pub(crate) content_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct CanonicalElementV2 {
    pub(crate) element_id: String,
    pub(crate) frame_id: String,
    pub(crate) bounds: Option<PacketBoundsV2>,
    pub(crate) text_reference: Option<String>,
    pub(crate) visual_description: Option<String>,
    pub(crate) native_role: Option<String>,
    pub(crate) native_subrole: Option<String>,
    pub(crate) native_actionability: bool,
    pub(crate) region_role: RegionRoleV2,
    pub(crate) focused: bool,
    pub(crate) editable: bool,
    pub(crate) selected: bool,
    pub(crate) interactive: bool,
    pub(crate) parent_element_id: Option<String>,
    pub(crate) child_element_ids: Vec<String>,
    pub(crate) source_votes: Vec<String>,
    pub(crate) source_conflicts: Vec<String>,
    pub(crate) first_seen_at_ms: i64,
    pub(crate) changed_at_ms: i64,
    pub(crate) authorship_status: AuthorshipStatusV2,
    pub(crate) causal_evidence_refs: Vec<EvidenceHandleV2>,
    pub(crate) task_eligible: bool,
    pub(crate) rejection_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct KeyframeReferenceV2 {
    pub(crate) frame_id: String,
    pub(crate) observed_at_ms: i64,
    pub(crate) partition: EvidencePartitionV2,
    pub(crate) privacy_status: String,
    pub(crate) model_eligible: bool,
    pub(crate) local_image_handle_hash: Option<String>,
    /// Available only while handling the explicit Continue request. The local path is
    /// deliberately omitted from serialization so checkpoints and audits retain only
    /// the hash above.
    #[serde(skip)]
    pub(crate) ephemeral_local_image_path: Option<String>,
    pub(crate) selection_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ActiveSurfaceIdentityV2 {
    pub(crate) app_name: Option<String>,
    pub(crate) app_bundle_id: Option<String>,
    pub(crate) window_title_hash: Option<String>,
    pub(crate) window_id: Option<i64>,
    pub(crate) browser_url_hash: Option<String>,
    pub(crate) document_path_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CausalEventV2 {
    pub(crate) event_id: String,
    pub(crate) event_kind: String,
    pub(crate) observed_at_ms: i64,
    pub(crate) frame_id: String,
    pub(crate) partition: EvidencePartitionV2,
    pub(crate) causal_parent_ids: Vec<String>,
    pub(crate) committed: Option<bool>,
    pub(crate) source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct FrameChangeV2 {
    pub(crate) frame_id: String,
    pub(crate) diff_kind: Option<String>,
    pub(crate) summary_hash: Option<String>,
    pub(crate) added_text_hashes: Option<String>,
    pub(crate) removed_text_hashes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct PacketSizeAccountingV2 {
    pub(crate) frame_count: usize,
    pub(crate) keyframe_count: usize,
    pub(crate) canonical_element_count: usize,
    pub(crate) causal_event_count: usize,
    pub(crate) serialized_bytes: usize,
    pub(crate) estimated_tokens: usize,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ObservationPacketV2 {
    pub(crate) schema: String,
    pub(crate) packet_id: String,
    pub(crate) observed_at_ms: i64,
    pub(crate) session_id: Option<String>,
    pub(crate) evidence_watermark: String,
    pub(crate) active_surface: ActiveSurfaceIdentityV2,
    pub(crate) current_frame: KeyframeReferenceV2,
    pub(crate) semantic_keyframes: Vec<KeyframeReferenceV2>,
    pub(crate) canonical_elements: Vec<CanonicalElementV2>,
    pub(crate) focused_element_ids: Vec<String>,
    pub(crate) editable_element_ids: Vec<String>,
    pub(crate) selected_element_ids: Vec<String>,
    pub(crate) causal_events: Vec<CausalEventV2>,
    pub(crate) frame_changes: Vec<FrameChangeV2>,
    pub(crate) capture_trigger_ids: Vec<String>,
    pub(crate) transition_ids: Vec<String>,
    pub(crate) return_anchor_facts: Vec<EvidenceHandleV2>,
    pub(crate) previous_valid_snapshot_id: Option<String>,
    pub(crate) evidence_quality: String,
    pub(crate) missing_source_notes: Vec<String>,
    pub(crate) conflicting_observations: Vec<String>,
    pub(crate) partitions: BTreeMap<EvidencePartitionV2, Vec<String>>,
    pub(crate) size: PacketSizeAccountingV2,
}

pub(crate) fn is_private_status(status: Option<&str>) -> bool {
    let status = status.unwrap_or("unknown").trim().to_ascii_lowercase();
    matches!(
        status.as_str(),
        "private" | "blocked" | "secure" | "sensitive" | "denied" | "redacted"
    ) || status.contains("private")
        || status.contains("blocked")
        || status.contains("secure")
}

fn hash_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| stable_hash(value.as_bytes()))
}

fn normalize_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn partition_frames(frames: &[EvidenceFrame]) -> BTreeMap<String, EvidencePartitionV2> {
    let Some(current) = frames.last() else {
        return BTreeMap::new();
    };
    let current_surface = (
        current
            .app_bundle_id
            .as_deref()
            .or(current.app_name.as_deref()),
        current.window_id,
    );
    let mut result = BTreeMap::new();
    for (index, frame) in frames.iter().enumerate() {
        let surface = (
            frame.app_bundle_id.as_deref().or(frame.app_name.as_deref()),
            frame.window_id,
        );
        let partition = if frame.id == current.id {
            EvidencePartitionV2::Current
        } else if surface == current_surface {
            EvidencePartitionV2::Prior
        } else if index + 3 >= frames.len()
            && (frame.focused_node_evidence || !frame.typing_bursts.is_empty())
        {
            EvidencePartitionV2::Support
        } else {
            EvidencePartitionV2::Background
        };
        result.insert(frame.id.clone(), partition);
    }
    result
}

fn boundary_reasons(frame: &EvidenceFrame) -> Vec<String> {
    let mut reasons = BTreeSet::new();
    let trigger = frame.capture_trigger.to_ascii_lowercase();
    for (needle, reason) in [
        ("submit", "submit_boundary"),
        ("enter", "submit_boundary"),
        ("send", "submit_boundary"),
        ("switch", "surface_switch_boundary"),
        ("focus", "focus_boundary"),
        ("command", "command_boundary"),
        ("error", "visible_error_boundary"),
        ("idle", "idle_after_progress_boundary"),
        ("manual", "manual_continue_boundary"),
    ] {
        if trigger.contains(needle) {
            reasons.insert(reason.to_string());
        }
    }
    for event in &frame.ui_events {
        let kind = event.event_type.to_ascii_lowercase();
        if kind.contains("switch") || kind.contains("focus") {
            reasons.insert("surface_switch_boundary".into());
        }
        if kind.contains("submit") || kind.contains("command") {
            reasons.insert("causal_action_boundary".into());
        }
    }
    if frame.typing_bursts.iter().any(|burst| burst.committed) {
        reasons.insert("committed_typing_boundary".into());
    }
    if frame.frame_diff.is_some() {
        reasons.insert("material_change_boundary".into());
    }
    if frame.transition.is_some() {
        reasons.insert("event_transition_boundary".into());
    }
    reasons.into_iter().collect()
}

fn keyframe_reference(
    frame: &EvidenceFrame,
    partition: EvidencePartitionV2,
    mut reasons: Vec<String>,
) -> KeyframeReferenceV2 {
    reasons.sort();
    reasons.dedup();
    let private = is_private_status(frame.privacy_status.as_deref());
    KeyframeReferenceV2 {
        frame_id: frame.id.clone(),
        observed_at_ms: frame.captured_at,
        partition,
        privacy_status: frame
            .privacy_status
            .clone()
            .unwrap_or_else(|| "unknown".into()),
        model_eligible: !private,
        local_image_handle_hash: (!private)
            .then(|| hash_optional(frame.active_window_crop_path.as_deref()))
            .flatten(),
        ephemeral_local_image_path: (!private)
            .then(|| frame.active_window_crop_path.clone())
            .flatten(),
        selection_reasons: reasons,
    }
}

fn select_keyframes(
    frames: &[EvidenceFrame],
    partitions: &BTreeMap<String, EvidencePartitionV2>,
) -> Vec<KeyframeReferenceV2> {
    let mut scored = frames
        .iter()
        .map(|frame| {
            let reasons = boundary_reasons(frame);
            let boundary_score = reasons.len() as i64;
            let semantic_change = i64::from(frame.frame_diff.is_some())
                + i64::from(frame.transition.is_some())
                + i64::from(frame.typing_bursts.iter().any(|burst| burst.committed));
            (frame, reasons, boundary_score * 10 + semantic_change * 4)
        })
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| {
        right
            .2
            .cmp(&left.2)
            .then_with(|| right.0.captured_at.cmp(&left.0.captured_at))
            .then_with(|| left.0.id.cmp(&right.0.id))
    });
    let current_id = frames.last().map(|frame| frame.id.as_str());
    let mut selected = Vec::new();
    for (frame, mut reasons, _) in scored {
        if selected.len() >= MAX_KEYFRAMES {
            break;
        }
        if reasons.is_empty() && Some(frame.id.as_str()) != current_id {
            continue;
        }
        if Some(frame.id.as_str()) == current_id {
            reasons.push("current_frame".into());
        }
        selected.push(keyframe_reference(
            frame,
            partitions
                .get(&frame.id)
                .copied()
                .unwrap_or(EvidencePartitionV2::Background),
            reasons,
        ));
    }
    if let Some(current) = frames.last() {
        if !selected.iter().any(|item| item.frame_id == current.id) {
            selected.push(keyframe_reference(
                current,
                EvidencePartitionV2::Current,
                vec!["current_frame".into()],
            ));
        }
    }
    if selected.len() < 2 && frames.len() > 1 {
        if let Some(baseline) = frames
            .iter()
            .rev()
            .find(|frame| !selected.iter().any(|item| item.frame_id == frame.id))
        {
            selected.push(keyframe_reference(
                baseline,
                partitions
                    .get(&baseline.id)
                    .copied()
                    .unwrap_or(EvidencePartitionV2::Prior),
                vec!["causal_baseline_for_current_observation".into()],
            ));
        }
    }
    selected.sort_by_key(|item| (item.observed_at_ms, item.frame_id.clone()));
    selected
}

fn rect_overlap(left: Option<&PacketBoundsV2>, right: Option<&PacketBoundsV2>) -> bool {
    let (Some(left), Some(right)) = (left, right) else {
        return false;
    };
    let x_overlap = left.x < right.x + right.width && right.x < left.x + left.width;
    let y_overlap = left.y < right.y + right.height && right.y < left.y + left.height;
    x_overlap && y_overlap
}

fn role_for(hint: &str) -> RegionRoleV2 {
    let hint = hint.to_ascii_lowercase();
    if hint.contains("browser_chrome")
        || hint.contains("address_bar")
        || hint.contains("tab_chrome")
    {
        RegionRoleV2::BrowserChrome
    } else if hint.contains("navigation") || hint.contains("menu") || hint.contains("tab") {
        RegionRoleV2::Navigation
    } else if hint.contains("toolbar") {
        RegionRoleV2::Toolbar
    } else if hint.contains("button") || hint.contains("control") || hint.contains("picker") {
        RegionRoleV2::Control
    } else if hint.contains("composer") || hint.contains("editor") || hint.contains("text_field") {
        RegionRoleV2::ComposerEditor
    } else if hint.contains("terminal_input") {
        RegionRoleV2::TerminalInput
    } else if hint.contains("terminal_output") || hint.contains("terminal") {
        RegionRoleV2::TerminalOutput
    } else if hint.contains("user") || hint.contains("authored") {
        RegionRoleV2::UserAuthoredContent
    } else if hint.contains("assistant") || hint.contains("agent") || hint.contains("output") {
        RegionRoleV2::ApplicationAgentOutput
    } else if hint.contains("sidebar") {
        RegionRoleV2::Sidebar
    } else if hint.contains("modal") || hint.contains("dialog") {
        RegionRoleV2::Modal
    } else if hint.contains("notification") {
        RegionRoleV2::Notification
    } else if hint.contains("status") {
        RegionRoleV2::Status
    } else if hint.contains("document") || hint.contains("canvas") {
        RegionRoleV2::DocumentCanvas
    } else if hint.contains("content") {
        RegionRoleV2::PrimaryContent
    } else {
        RegionRoleV2::Unknown
    }
}

fn control_role(role: RegionRoleV2) -> bool {
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

fn element_from_unit(frame: &EvidenceFrame, unit: &EvidenceContentUnit) -> CanonicalElementV2 {
    let hint = format!(
        "{} {} {}",
        unit.unit_type,
        unit.semantic_role.as_deref().unwrap_or(""),
        unit.ownership_kind.as_deref().unwrap_or("")
    );
    let region_role = role_for(&hint);
    let task_eligible = !control_role(region_role) && !is_categorical_control_hint(&hint);
    let text_reference = unit
        .text_hash
        .clone()
        .or_else(|| hash_optional(unit.text.as_deref()));
    CanonicalElementV2 {
        element_id: format!("element:{}:{}", frame.id, unit.id),
        frame_id: frame.id.clone(),
        bounds: unit.bounds.map(Into::into),
        text_reference,
        visual_description: None,
        native_role: Some(unit.unit_type.clone()),
        native_subrole: unit.semantic_role.clone(),
        native_actionability: control_role(region_role) || is_categorical_control_hint(&hint),
        region_role,
        focused: frame.focused_node_evidence && unit.semantic_role.as_deref() == Some("focused"),
        editable: matches!(
            region_role,
            RegionRoleV2::ComposerEditor | RegionRoleV2::TerminalInput
        ),
        selected: frame.selected_text_present,
        interactive: control_role(region_role)
            || is_categorical_control_hint(&hint)
            || matches!(
                region_role,
                RegionRoleV2::ComposerEditor | RegionRoleV2::TerminalInput
            ),
        parent_element_id: None,
        child_element_ids: Vec::new(),
        source_votes: vec![unit.source.clone()],
        source_conflicts: Vec::new(),
        first_seen_at_ms: frame.captured_at,
        changed_at_ms: frame.captured_at,
        authorship_status: match region_role {
            RegionRoleV2::UserAuthoredContent | RegionRoleV2::TerminalInput => {
                AuthorshipStatusV2::User
            }
            RegionRoleV2::ApplicationAgentOutput | RegionRoleV2::TerminalOutput => {
                AuthorshipStatusV2::ApplicationOrAgent
            }
            _ => AuthorshipStatusV2::Unknown,
        },
        causal_evidence_refs: Vec::new(),
        task_eligible,
        rejection_reasons: (!task_eligible)
            .then(|| vec!["categorical_control_ineligible".into()])
            .unwrap_or_default(),
    }
}

fn merge_ocr(element: &mut CanonicalElementV2, frame: &EvidenceFrame, ocr: &EvidenceOcrSpan) {
    if !element.source_votes.iter().any(|vote| vote == "ocr") {
        element.source_votes.push("ocr".into());
    }
    let ocr_hash = stable_hash(normalize_text(&ocr.text).as_bytes());
    if element
        .text_reference
        .as_deref()
        .is_some_and(|hash| hash != ocr_hash)
    {
        element
            .source_conflicts
            .push("ax_ocr_text_disagreement".into());
    }
    element.changed_at_ms = element.changed_at_ms.max(frame.captured_at);
}

fn canonical_elements(frames: &[EvidenceFrame]) -> Vec<CanonicalElementV2> {
    let mut elements = Vec::new();
    for frame in frames {
        for unit in &frame.content_units {
            if elements.len() >= MAX_ELEMENTS {
                break;
            }
            elements.push(element_from_unit(frame, unit));
        }
        for ocr in &frame.ocr_spans {
            let normalized = normalize_text(&ocr.text);
            let ocr_bounds = Some(PacketBoundsV2::from(ocr.bounds));
            let matching = elements.iter_mut().find(|element| {
                element.frame_id == frame.id
                    && (element.text_reference.as_deref()
                        == Some(stable_hash(normalized.as_bytes()).as_str())
                        || rect_overlap(element.bounds.as_ref(), ocr_bounds.as_ref()))
            });
            if let Some(element) = matching {
                merge_ocr(element, frame, ocr);
            } else if elements.len() < MAX_ELEMENTS {
                elements.push(CanonicalElementV2 {
                    element_id: format!("element:{}:ocr:{}", frame.id, ocr.id),
                    frame_id: frame.id.clone(),
                    bounds: ocr_bounds,
                    text_reference: Some(stable_hash(normalized.as_bytes())),
                    visual_description: None,
                    native_role: Some("ocr_span".into()),
                    native_subrole: None,
                    native_actionability: false,
                    region_role: RegionRoleV2::Unknown,
                    focused: false,
                    editable: false,
                    selected: false,
                    interactive: false,
                    parent_element_id: None,
                    child_element_ids: Vec::new(),
                    source_votes: vec!["ocr".into()],
                    source_conflicts: Vec::new(),
                    first_seen_at_ms: frame.captured_at,
                    changed_at_ms: frame.captured_at,
                    authorship_status: AuthorshipStatusV2::Unknown,
                    causal_evidence_refs: Vec::new(),
                    task_eligible: true,
                    rejection_reasons: Vec::new(),
                });
            }
        }
    }
    elements.sort_by(|left, right| left.element_id.cmp(&right.element_id));
    elements
}

fn causal_events(
    frames: &[EvidenceFrame],
    partitions: &BTreeMap<String, EvidencePartitionV2>,
) -> Vec<CausalEventV2> {
    let mut events = Vec::new();
    for frame in frames {
        let partition = partitions
            .get(&frame.id)
            .copied()
            .unwrap_or(EvidencePartitionV2::Background);
        for event in &frame.ui_events {
            if events.len() >= MAX_CAUSAL_EVENTS {
                break;
            }
            events.push(CausalEventV2 {
                event_id: event.id.clone(),
                event_kind: event.event_type.clone(),
                observed_at_ms: event.ts_ms.unwrap_or(frame.captured_at),
                frame_id: frame.id.clone(),
                partition,
                causal_parent_ids: Vec::new(),
                committed: None,
                source: "ui_event".into(),
            });
        }
        for burst in &frame.typing_bursts {
            if events.len() >= MAX_CAUSAL_EVENTS {
                break;
            }
            events.push(CausalEventV2 {
                event_id: burst.id.clone(),
                event_kind: burst
                    .commit_signal
                    .clone()
                    .unwrap_or_else(|| "typing_burst".into()),
                observed_at_ms: frame.captured_at,
                frame_id: frame.id.clone(),
                partition,
                causal_parent_ids: frame
                    .trigger
                    .as_ref()
                    .map(|trigger| trigger.caused_by_event_ids.clone())
                    .unwrap_or_default(),
                committed: Some(burst.committed),
                source: "typing_burst".into(),
            });
        }
    }
    events.sort_by(|left, right| {
        left.observed_at_ms
            .cmp(&right.observed_at_ms)
            .then_with(|| left.event_id.cmp(&right.event_id))
    });
    events
}

pub(super) fn build_observation_packet(
    frames: &[EvidenceFrame],
    evidence_watermark: &str,
    previous_valid_snapshot_id: Option<String>,
) -> Result<ObservationPacketV2, String> {
    let Some(current) = frames.last() else {
        return Err("observation packet requires at least one evidence frame".into());
    };
    let partitions_by_frame = partition_frames(frames);
    let semantic_keyframes = select_keyframes(frames, &partitions_by_frame);
    let current_frame = semantic_keyframes
        .iter()
        .find(|keyframe| keyframe.frame_id == current.id)
        .cloned()
        .unwrap_or_else(|| {
            keyframe_reference(
                current,
                EvidencePartitionV2::Current,
                vec!["current_frame".into()],
            )
        });
    let canonical_elements = canonical_elements(frames);
    let causal_events = causal_events(frames, &partitions_by_frame);
    let frame_changes = frames
        .iter()
        .filter_map(|frame| {
            frame.frame_diff.as_ref().map(|diff| FrameChangeV2 {
                frame_id: frame.id.clone(),
                diff_kind: diff.diff_type.clone(),
                summary_hash: hash_optional(diff.summary.as_deref()),
                added_text_hashes: diff.added_text_hashes.clone(),
                removed_text_hashes: diff.removed_text_hashes.clone(),
            })
        })
        .collect::<Vec<_>>();
    let capture_trigger_ids = frames
        .iter()
        .filter_map(|frame| frame.trigger.as_ref().map(|trigger| trigger.id.clone()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let transition_ids = frames
        .iter()
        .filter_map(|frame| {
            frame
                .transition
                .as_ref()
                .map(|transition| transition.id.clone())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let focused_element_ids = canonical_elements
        .iter()
        .filter(|element| element.focused)
        .map(|element| element.element_id.clone())
        .collect();
    let editable_element_ids = canonical_elements
        .iter()
        .filter(|element| element.editable)
        .map(|element| element.element_id.clone())
        .collect();
    let selected_element_ids = canonical_elements
        .iter()
        .filter(|element| element.selected)
        .map(|element| element.element_id.clone())
        .collect();
    let mut partitions = BTreeMap::<EvidencePartitionV2, Vec<String>>::new();
    for (frame_id, partition) in &partitions_by_frame {
        partitions
            .entry(*partition)
            .or_default()
            .push(frame_id.clone());
    }
    let private_count = semantic_keyframes
        .iter()
        .filter(|keyframe| !keyframe.model_eligible)
        .count();
    let mut missing_source_notes = Vec::new();
    if current.content_units.is_empty() {
        missing_source_notes.push("current_frame_missing_content_units".into());
    }
    if current.ocr_spans.is_empty() {
        missing_source_notes.push("current_frame_missing_ocr".into());
    }
    if current.trigger.is_none() {
        missing_source_notes.push("current_frame_missing_capture_trigger".into());
    }
    if private_count > 0 {
        missing_source_notes.push("private_keyframe_model_ineligible".into());
    }
    missing_source_notes.truncate(MAX_NOTES);
    let conflicting_observations = canonical_elements
        .iter()
        .flat_map(|element| element.source_conflicts.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(MAX_NOTES)
        .collect::<Vec<_>>();
    let packet_seed = format!(
        "{}:{}:{}",
        current.session_id.as_deref().unwrap_or("no_session"),
        evidence_watermark,
        current.id
    );
    let mut packet = ObservationPacketV2 {
        schema: OBSERVATION_PACKET_SCHEMA_V2.into(),
        packet_id: format!("packet-{}", stable_hash(packet_seed.as_bytes())),
        observed_at_ms: current.captured_at,
        session_id: current.session_id.clone(),
        evidence_watermark: evidence_watermark.into(),
        active_surface: ActiveSurfaceIdentityV2 {
            app_name: current.app_name.clone(),
            app_bundle_id: current.app_bundle_id.clone(),
            window_title_hash: hash_optional(current.window_name.as_deref()),
            window_id: current.window_id,
            browser_url_hash: hash_optional(current.browser_url.as_deref()),
            document_path_hash: hash_optional(current.document_path.as_deref()),
        },
        current_frame,
        semantic_keyframes,
        canonical_elements,
        focused_element_ids,
        editable_element_ids,
        selected_element_ids,
        causal_events,
        frame_changes,
        capture_trigger_ids,
        transition_ids,
        return_anchor_facts: [
            ("browser_url", current.browser_url.as_deref()),
            ("document_path", current.document_path.as_deref()),
        ]
        .into_iter()
        .filter_map(|(kind, value)| {
            hash_optional(value).map(|content_hash| EvidenceHandleV2 {
                source_kind: format!("return_anchor_fact:{kind}"),
                record_id: format!("{}:{kind}", current.id),
                frame_id: Some(current.id.clone()),
                content_hash: Some(content_hash),
            })
        })
        .collect(),
        previous_valid_snapshot_id,
        evidence_quality: if private_count > 0 {
            "privacy_limited".into()
        } else if frames.len() >= 2 {
            "bounded_multisource".into()
        } else {
            "thin".into()
        },
        missing_source_notes,
        conflicting_observations,
        partitions,
        size: PacketSizeAccountingV2 {
            frame_count: frames.len(),
            keyframe_count: 0,
            canonical_element_count: 0,
            causal_event_count: 0,
            serialized_bytes: 0,
            estimated_tokens: 0,
            truncated: frames.len() > 24,
        },
    };
    packet.size.keyframe_count = packet.semantic_keyframes.len();
    packet.size.canonical_element_count = packet.canonical_elements.len();
    packet.size.causal_event_count = packet.causal_events.len();
    for _ in 0..3 {
        let bytes = serde_json::to_vec(&packet).map_err(|error| error.to_string())?;
        packet.size.serialized_bytes = bytes.len();
        packet.size.estimated_tokens = bytes.len().div_ceil(4);
    }
    Ok(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuation::{
        EvidenceFrameDiff, EvidenceOcrSpan, EvidenceTransition, EvidenceTrigger,
        EvidenceTypingBurst, EvidenceUiEvent,
    };

    fn frame(id: &str, observed_at_ms: i64, trigger: &str) -> EvidenceFrame {
        EvidenceFrame {
            id: id.into(),
            captured_at: observed_at_ms,
            app_name: Some("Test App".into()),
            window_name: Some("Window".into()),
            browser_url: None,
            document_path: None,
            capture_trigger: trigger.into(),
            text_source: Some("accessibility".into()),
            scope: Some("active_window".into()),
            window_id: Some(1),
            active_window_crop_path: Some("/private/local/short-lived.jpg".into()),
            full_text: None,
            active_text: None,
            background_text: None,
            full_text_quality: Some("structured".into()),
            text_quality_flags: Vec::new(),
            structured_semantic_text: None,
            content_hash: Some(format!("hash-{id}")),
            image_hash: Some(format!("image-{id}")),
            privacy_status: Some("allowed".into()),
            app_bundle_id: Some("com.example.test".into()),
            previous_frame_id: None,
            session_id: Some("session-test".into()),
            app_contexts: Vec::new(),
            content_units: Vec::new(),
            ocr_spans: Vec::new(),
            visible_windows: Vec::new(),
            ui_events: Vec::new(),
            trigger: None,
            transition: None,
            frame_diff: None,
            typing_bursts: Vec::new(),
            clipboard_events: Vec::new(),
            focused_node_evidence: false,
            selected_text_present: false,
        }
    }

    fn content_unit(id: &str, role: &str, text: &str, bounds: Rect) -> EvidenceContentUnit {
        EvidenceContentUnit {
            id: id.into(),
            source: "ax".into(),
            unit_type: role.into(),
            semantic_role: Some(role.into()),
            text: Some(text.into()),
            text_hash: Some(stable_hash(normalize_text(text).as_bytes())),
            confidence: Some(0.9),
            ocr_span_ids: Vec::new(),
            bounds: Some(bounds),
            source_scope: Some("active".into()),
            ownership_kind: None,
            ownership_confidence: Some(0.8),
            active_artifact_match_confidence: Some(0.8),
            owner_window_id: Some(1),
            owner_bundle_id: Some("com.example.test".into()),
            quality_flags: Vec::new(),
        }
    }

    #[test]
    fn same_watermark_produces_byte_stable_packet_semantics() {
        let mut current = frame("2", 2_000, "submit");
        current.typing_bursts.push(EvidenceTypingBurst {
            id: "burst-1".into(),
            enter_count: 1,
            paste_count: 0,
            committed: true,
            commit_signal: Some("enter".into()),
        });
        let frames = vec![frame("1", 1_000, "periodic"), current];
        let first = build_observation_packet(&frames, "watermark-1", None).unwrap();
        let second = build_observation_packet(&frames, "watermark-1", None).unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert_eq!(first.packet_id, second.packet_id);
    }

    #[test]
    fn semantic_boundaries_beat_unchanged_recency_for_keyframes() {
        let mut submit = frame("submit", 1_000, "submit");
        submit.trigger = Some(EvidenceTrigger {
            id: "trigger-submit".into(),
            trigger_type: "submit".into(),
            caused_by_event_ids: vec!["event-submit".into()],
        });
        let mut switched = frame("switch", 2_000, "window_switch");
        switched.ui_events.push(EvidenceUiEvent {
            id: "event-switch".into(),
            ts_ms: Some(2_000),
            event_type: "window_switch".into(),
            key_category: None,
        });
        let mut errored = frame("error", 3_000, "visible_error");
        errored.transition = Some(EvidenceTransition {
            id: "transition-error".into(),
            primary_event_id: None,
            transition_type: Some("error".into()),
            summary: None,
            confidence: Some(0.9),
        });
        let unchanged = frame("unchanged", 3_500, "periodic");
        let current = frame("current", 4_000, "periodic");
        let packet = build_observation_packet(
            &[submit, switched, errored, unchanged, current],
            "watermark",
            None,
        )
        .unwrap();
        let ids = packet
            .semantic_keyframes
            .iter()
            .map(|keyframe| keyframe.frame_id.as_str())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"submit"));
        assert!(ids.contains(&"switch"));
        assert!(ids.contains(&"error"));
        assert!(ids.contains(&"current"));
        assert!(!ids.contains(&"unchanged"));
    }

    #[test]
    fn private_frames_never_become_model_eligible() {
        let mut private = frame("private", 1_000, "manual");
        private.privacy_status = Some("private".into());
        let packet = build_observation_packet(&[private], "watermark", None).unwrap();
        assert!(!packet.current_frame.model_eligible);
        assert!(packet.current_frame.local_image_handle_hash.is_none());
        assert!(packet
            .semantic_keyframes
            .iter()
            .all(|keyframe| !keyframe.model_eligible));
    }

    #[test]
    fn ax_ocr_duplicates_merge_without_losing_conflict_or_provenance() {
        let bounds = Rect {
            x: 10.0,
            y: 10.0,
            w: 200.0,
            h: 40.0,
        };
        let mut current = frame("1", 1_000, "material_change");
        current
            .content_units
            .push(content_unit("ax-1", "content", "Run tests", bounds));
        current.ocr_spans.push(EvidenceOcrSpan {
            id: "ocr-1".into(),
            text: "Run all tests".into(),
            confidence: Some(0.9),
            bounds,
            source_scope: Some("active".into()),
            ownership_kind: None,
            ownership_confidence: Some(0.8),
            active_artifact_match_confidence: Some(0.8),
            owner_window_id: Some(1),
            owner_bundle_id: Some("com.example.test".into()),
            owner_app_name: Some("Test App".into()),
            owner_window_title: Some("Window".into()),
            quality_flags: Vec::new(),
        });
        current.frame_diff = Some(EvidenceFrameDiff {
            diff_type: Some("text_change".into()),
            added_text_hashes: None,
            removed_text_hashes: None,
            summary: Some("changed".into()),
        });
        let packet = build_observation_packet(&[current], "watermark", None).unwrap();
        assert_eq!(packet.canonical_elements.len(), 1);
        let element = &packet.canonical_elements[0];
        assert_eq!(element.source_votes, vec!["ax", "ocr"]);
        assert!(element
            .source_conflicts
            .contains(&"ax_ocr_text_disagreement".into()));
    }

    #[test]
    fn controls_are_categorically_task_ineligible() {
        let mut current = frame("1", 1_000, "manual");
        current.content_units.push(content_unit(
            "button-1",
            "button",
            "Approve for me",
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 30.0,
            },
        ));
        let packet = build_observation_packet(&[current], "watermark", None).unwrap();
        let control = &packet.canonical_elements[0];
        assert_eq!(control.region_role, RegionRoleV2::Control);
        assert!(!control.task_eligible);
        assert_eq!(
            control.rejection_reasons,
            vec!["categorical_control_ineligible"]
        );
    }
}
