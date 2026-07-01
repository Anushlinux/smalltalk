use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkstateMemory {
    pub session_id: String,
    pub active_workstate_id: Option<String>,
    pub active_workstream_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Surface {
    pub id: String,
    pub session_id: String,
    pub surface_key: String,
    pub surface_type: String,
    pub app_name: Option<String>,
    pub app_bundle_id: Option<String>,
    pub app_pid: Option<i64>,
    pub window_id: Option<i64>,
    pub window_title: Option<String>,
    pub url_ref: Option<String>,
    pub document_ref: Option<String>,
    pub privacy_state: String,
    pub confidence: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SurfaceState {
    pub id: String,
    pub surface_id: String,
    pub session_id: String,
    pub ts_ms: i64,
    pub surface_state: String,
    pub focused_object: Option<String>,
    pub visible_text_hash: Option<String>,
    pub visible_text_excerpt_redacted: Option<String>,
    pub selected_text_hash: Option<String>,
    pub selected_text_excerpt_redacted: Option<String>,
    pub viewport_signature: Option<String>,
    pub ax_state: String,
    pub ocr_state: String,
    pub last_event_id: Option<String>,
    pub last_frame_id: Option<String>,
    pub quality_flags: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Observation {
    pub id: String,
    pub session_id: String,
    pub ts_ms: i64,
    pub kind: String,
    pub source: String,
    pub surface_id: Option<String>,
    pub workstate_id: Option<String>,
    pub workstream_id: Option<String>,
    pub frame_id: Option<String>,
    pub event_ids: Vec<String>,
    pub summary: String,
    pub confidence: f64,
    pub payload: Value,
    pub privacy_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntentSignal {
    pub kind: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Workstream {
    pub id: String,
    pub session_id: String,
    pub status: String,
    pub primary_surface_id: Option<String>,
    pub current_focus_surface_id: Option<String>,
    pub label_hypothesis: String,
    pub current_activity_summary: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CutoffPoint {
    pub id: String,
    pub session_id: String,
    pub workstate_id: String,
    pub workstream_id: Option<String>,
    pub observation_id: String,
    pub surface_id: Option<String>,
    pub reason: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResumeAnchor {
    pub id: String,
    pub session_id: String,
    pub workstate_id: String,
    pub workstream_id: Option<String>,
    pub surface_id: Option<String>,
    pub anchor_type: String,
    pub value_redacted: String,
    pub frame_id: Option<String>,
    pub observation_id: Option<String>,
    pub confidence: f64,
    pub why_this_anchor: String,
    pub privacy_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceNeed {
    pub id: String,
    pub session_id: String,
    pub workstate_id: String,
    pub workstream_id: Option<String>,
    pub observation_id: Option<String>,
    pub surface_id: Option<String>,
    pub need_type: String,
    pub reason: String,
    pub modality: String,
    pub priority: i64,
    pub status: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceArtifact {
    pub id: String,
    pub session_id: String,
    pub workstate_id: Option<String>,
    pub workstream_id: Option<String>,
    pub surface_id: Option<String>,
    pub observation_id: Option<String>,
    pub evidence_need_id: Option<String>,
    pub kind: String,
    pub ref_table: String,
    pub ref_id: String,
    pub role: String,
    pub summary: String,
    pub privacy_state: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnswerabilityBundle {
    pub schema: String,
    pub session_id: String,
    pub current_focus: Option<AnswerabilityTarget>,
    pub current_activity: Option<AnswerabilityActivity>,
    pub resume_work_target: Option<AnswerabilityTarget>,
    pub return_target: Option<AnswerabilityTarget>,
    pub active_surfaces: Vec<AnswerabilitySurface>,
    pub workstreams: Vec<AnswerabilityWorkstream>,
    pub branch_timeline: Vec<AnswerabilityEdge>,
    pub candidate_cutoff_points: Vec<CutoffPoint>,
    pub candidate_resume_anchors: Vec<ResumeAnchor>,
    pub observations: Vec<Observation>,
    pub evidence_artifacts: Vec<EvidenceArtifact>,
    pub missing_evidence: Vec<String>,
    pub privacy_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnswerabilityTarget {
    pub evidence_handle: String,
    pub surface_id: Option<String>,
    pub app: Option<String>,
    pub title: Option<String>,
    pub surface_type: String,
    pub surface_state: String,
    pub anchor: Option<String>,
    pub reason: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnswerabilityActivity {
    pub evidence_handle: String,
    pub activity_type: String,
    pub summary: String,
    pub reason: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnswerabilitySurface {
    pub surface_id: String,
    pub surface_key: String,
    pub surface_type: String,
    pub app: Option<String>,
    pub title: Option<String>,
    pub surface_state: String,
    pub last_observed_at_ms: i64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnswerabilityWorkstream {
    pub workstream_id: String,
    pub status: String,
    pub primary_surface_id: Option<String>,
    pub current_focus_surface_id: Option<String>,
    pub current_activity_summary: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnswerabilityEdge {
    pub from_workstream_id: String,
    pub to_workstream_id: String,
    pub kind: String,
    pub reason: String,
    pub confidence: f64,
}
