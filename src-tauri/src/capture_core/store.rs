#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationRecord {
    pub ts_ms: i64,
    pub event_type: String,
    pub surface_key: String,
    pub frame_id: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceRecordKind {
    ObservationOnly,
    ImageBackedFrame,
}

pub fn evidence_record_kind(frame_id: Option<&str>) -> EvidenceRecordKind {
    if frame_id.is_some() {
        EvidenceRecordKind::ImageBackedFrame
    } else {
        EvidenceRecordKind::ObservationOnly
    }
}
