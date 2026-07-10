use serde::{Deserialize, Serialize};

pub const ACTIVITY_RECAP_SCHEMA: &str = "smalltalk.activity_recap.v1";

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityConfidence {
    #[default]
    None,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityEvidenceConfidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityCurrentState {
    ActivelyWorking,
    RecentlyDetoured,
    PausedAfterProgress,
    Blocked,
    CompleteOrIdle,
    #[default]
    Unclear,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityDetourRole {
    Support,
    Detour,
    Interrupt,
    CurrentFocusOnly,
    PromotedPrimary,
    Unclear,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivitySupportRole {
    SourceEvidence,
    BranchSupport,
    OutputVerification,
    Blocker,
    MessageInterrupt,
    Diagnostic,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityEvidenceAnchorType {
    Frame,
    Event,
    Action,
    Episode,
    Workstream,
    OpenLoop,
    Branch,
    SurfaceSnapshot,
    MemoryCell,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityEvidenceSource {
    Local,
    ModelValidated,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityRecapGeneratedBy {
    #[default]
    Local,
    ModelAssisted,
    Fallback,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityRecapValidationStatus {
    Valid,
    #[default]
    Thin,
    Rejected,
    Fallback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActivityDetourSummary {
    pub surface_title: Option<String>,
    pub app_name: Option<String>,
    pub role: ActivityDetourRole,
    pub activity_label: Option<String>,
    pub reason: String,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
    pub confidence: ActivityEvidenceConfidence,
    pub evidence_anchor_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActivitySupportSummary {
    pub summary: String,
    pub role: ActivitySupportRole,
    pub confidence: ActivityEvidenceConfidence,
    pub evidence_anchor_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActivityEvidenceSpan {
    pub claim_key: String,
    pub claim_text: String,
    pub anchor_type: ActivityEvidenceAnchorType,
    pub anchor_ids: Vec<String>,
    pub confidence: ActivityEvidenceConfidence,
    pub source: ActivityEvidenceSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ContinueActivityRecap {
    pub schema: String,
    pub primary_work_summary: Option<String>,
    pub primary_work_label: Option<String>,
    pub primary_where_summary: Option<String>,
    pub activity_confidence: ActivityConfidence,
    pub target_confidence: ActivityConfidence,
    pub current_state: ActivityCurrentState,
    pub last_meaningful_state: Option<String>,
    pub unfinished_state: Option<String>,
    pub next_action_summary: Option<String>,
    pub recent_detours: Vec<ActivityDetourSummary>,
    pub supporting_context: Vec<ActivitySupportSummary>,
    pub why_this_target: Option<String>,
    pub why_no_safe_target: Option<String>,
    pub missing_evidence: Vec<String>,
    pub warnings: Vec<String>,
    pub evidence_spans: Vec<ActivityEvidenceSpan>,
    pub generated_by: ActivityRecapGeneratedBy,
    pub validation_status: ActivityRecapValidationStatus,
}

impl Default for ContinueActivityRecap {
    fn default() -> Self {
        Self {
            schema: ACTIVITY_RECAP_SCHEMA.to_string(),
            primary_work_summary: None,
            primary_work_label: None,
            primary_where_summary: None,
            activity_confidence: ActivityConfidence::None,
            target_confidence: ActivityConfidence::None,
            current_state: ActivityCurrentState::Unclear,
            last_meaningful_state: None,
            unfinished_state: None,
            next_action_summary: None,
            recent_detours: Vec::new(),
            supporting_context: Vec::new(),
            why_this_target: None,
            why_no_safe_target: None,
            missing_evidence: Vec::new(),
            warnings: Vec::new(),
            evidence_spans: Vec::new(),
            generated_by: ActivityRecapGeneratedBy::Local,
            validation_status: ActivityRecapValidationStatus::Thin,
        }
    }
}

impl ContinueActivityRecap {
    /// Applies the public-copy boundary without changing diagnostic anchor ids.
    /// Later P5 builders should call this after assembling evidence-backed claims.
    pub fn sanitized(mut self) -> Self {
        self.schema = ACTIVITY_RECAP_SCHEMA.to_string();
        self.primary_work_summary = sanitize_optional_text(self.primary_work_summary, 280);
        self.primary_work_label = sanitize_optional_text(self.primary_work_label, 120);
        self.primary_where_summary = sanitize_optional_text(self.primary_where_summary, 180);
        self.last_meaningful_state = sanitize_optional_text(self.last_meaningful_state, 240);
        self.unfinished_state = sanitize_optional_text(self.unfinished_state, 240);
        self.next_action_summary = sanitize_optional_text(self.next_action_summary, 240);
        self.why_this_target = sanitize_optional_text(self.why_this_target, 240);
        self.why_no_safe_target = sanitize_optional_text(self.why_no_safe_target, 240);
        self.missing_evidence = sanitize_text_list(self.missing_evidence, 180);
        self.warnings = sanitize_text_list(self.warnings, 180);
        self.recent_detours = self
            .recent_detours
            .into_iter()
            .filter_map(sanitize_detour)
            .collect();
        self.supporting_context = self
            .supporting_context
            .into_iter()
            .filter_map(sanitize_support)
            .collect();
        self.evidence_spans = self
            .evidence_spans
            .into_iter()
            .filter_map(sanitize_evidence_span)
            .collect();
        self
    }
}

fn sanitize_detour(mut detour: ActivityDetourSummary) -> Option<ActivityDetourSummary> {
    detour.surface_title = sanitize_optional_text(detour.surface_title, 160);
    detour.app_name = sanitize_optional_text(detour.app_name, 80);
    detour.activity_label = sanitize_optional_text(detour.activity_label, 120);
    detour.reason = sanitize_public_text(detour.reason, 220)?;
    Some(detour)
}

fn sanitize_support(mut support: ActivitySupportSummary) -> Option<ActivitySupportSummary> {
    support.summary = sanitize_public_text(support.summary, 220)?;
    Some(support)
}

fn sanitize_evidence_span(mut span: ActivityEvidenceSpan) -> Option<ActivityEvidenceSpan> {
    span.claim_key = sanitize_public_text(span.claim_key, 80)?;
    span.claim_text = sanitize_public_text(span.claim_text, 280)?;
    Some(span)
}

fn sanitize_optional_text(value: Option<String>, max_chars: usize) -> Option<String> {
    value.and_then(|text| sanitize_public_text(text, max_chars))
}

fn sanitize_text_list(values: Vec<String>, max_chars: usize) -> Vec<String> {
    let mut sanitized = Vec::new();
    for value in values {
        if let Some(value) = sanitize_public_text(value, max_chars) {
            if !sanitized.contains(&value) {
                sanitized.push(value);
            }
        }
    }
    sanitized
}

pub(super) fn sanitize_public_text(value: String, max_chars: usize) -> Option<String> {
    let original = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let original = original.trim();
    if original.is_empty() || contains_private_locator(original) || contains_internal_id(original) {
        return None;
    }
    let one_line = super::scrub_sensitive_text(&value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let trimmed = one_line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut output = trimmed.chars().take(max_chars).collect::<String>();
    if trimmed.chars().count() > max_chars {
        output = output.trim_end_matches([' ', '.', ',']).to_string();
        output.push_str("...");
    }
    Some(output)
}

/// Returns true only when model-produced public copy passes the same locator,
/// internal-id, redaction, and length boundary as deterministic recap copy.
/// Validation uses this before any model text can replace a local claim.
pub(crate) fn model_public_text_is_safe(value: &str, max_chars: usize) -> bool {
    if value.chars().count() > max_chars {
        return false;
    }
    sanitize_public_text(value.to_string(), max_chars).as_deref() == Some(value.trim())
}

fn contains_private_locator(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("://")
        || lower.contains("www.")
        || lower.contains("file:")
        || lower.contains("/users/")
        || lower.contains("/private/")
        || lower.contains("\\")
        || lower.split_whitespace().any(|token| {
            let token = token.trim_matches(|ch: char| matches!(ch, '(' | ')' | '[' | ']' | ','));
            token.starts_with('/') || token.starts_with("~/")
        })
}

fn contains_internal_id(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("continue-candidate-")
        || lower.contains("candidate-")
        || lower.contains("workstream-")
        || lower.contains("artifact-")
        || lower.contains("continue-action-")
        || lower.contains("action-")
        || lower.contains("episode-")
        || lower.contains("open-loop-")
        || lower.contains("branch-")
        || lower.contains("segment-")
        || lower.contains("moment-")
        || lower.contains("snapshot-")
        || lower.contains("memory-cell-")
        || lower.contains("frame-fallback")
        || lower.contains("frame_fallback")
        || lower.contains("action_id")
        || lower.contains("action id")
        || lower.contains("episode_id")
        || lower.contains("episode id")
        || lower.contains("branch_id")
        || lower.contains("branch id")
        || lower.contains("frame_id")
        || lower.contains("frame id")
        || lower.split_whitespace().any(|token| {
            token
                .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-')
                .strip_prefix("frame-")
                .is_some_and(|suffix| !suffix.is_empty())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Deserialize)]
    struct DecisionRecapEnvelope {
        #[serde(default)]
        activity_recap: ContinueActivityRecap,
    }

    #[test]
    fn default_activity_recap_matches_v1_contract() {
        let value = serde_json::to_value(ContinueActivityRecap::default()).unwrap();

        assert_eq!(
            value,
            json!({
                "schema": "smalltalk.activity_recap.v1",
                "primary_work_summary": null,
                "primary_work_label": null,
                "primary_where_summary": null,
                "activity_confidence": "none",
                "target_confidence": "none",
                "current_state": "unclear",
                "last_meaningful_state": null,
                "unfinished_state": null,
                "next_action_summary": null,
                "recent_detours": [],
                "supporting_context": [],
                "why_this_target": null,
                "why_no_safe_target": null,
                "missing_evidence": [],
                "warnings": [],
                "evidence_spans": [],
                "generated_by": "local",
                "validation_status": "thin"
            })
        );
    }

    #[test]
    fn partial_activity_recap_deserializes_with_safe_defaults() {
        let recap: ContinueActivityRecap = serde_json::from_value(json!({
            "schema": "smalltalk.activity_recap.v1",
            "primary_work_summary": "Reviewing the activity recap contract"
        }))
        .unwrap();

        assert_eq!(
            recap.primary_work_summary.as_deref(),
            Some("Reviewing the activity recap contract")
        );
        assert_eq!(recap.activity_confidence, ActivityConfidence::None);
        assert_eq!(recap.target_confidence, ActivityConfidence::None);
        assert_eq!(recap.current_state, ActivityCurrentState::Unclear);
        assert_eq!(recap.validation_status, ActivityRecapValidationStatus::Thin);

        let legacy: DecisionRecapEnvelope = serde_json::from_value(json!({})).unwrap();
        assert_eq!(legacy.activity_recap, ContinueActivityRecap::default());
    }

    #[test]
    fn populated_activity_recap_round_trips_without_collapsing_confidence() {
        let recap = ContinueActivityRecap {
            primary_work_summary: Some("Writing the P5 activity recap contract".to_string()),
            primary_work_label: Some("Writing contract".to_string()),
            primary_where_summary: Some("Smalltalk project".to_string()),
            activity_confidence: ActivityConfidence::High,
            target_confidence: ActivityConfidence::None,
            current_state: ActivityCurrentState::ActivelyWorking,
            supporting_context: vec![ActivitySupportSummary {
                summary: "The implementation brief defined the required fields".to_string(),
                role: ActivitySupportRole::SourceEvidence,
                confidence: ActivityEvidenceConfidence::High,
                evidence_anchor_ids: vec!["open-loop-1".to_string()],
            }],
            evidence_spans: vec![ActivityEvidenceSpan {
                claim_key: "primary_work".to_string(),
                claim_text: "Writing the P5 activity recap contract".to_string(),
                anchor_type: ActivityEvidenceAnchorType::OpenLoop,
                anchor_ids: vec!["open-loop-1".to_string()],
                confidence: ActivityEvidenceConfidence::High,
                source: ActivityEvidenceSource::Local,
            }],
            validation_status: ActivityRecapValidationStatus::Valid,
            ..ContinueActivityRecap::default()
        };

        let serialized = serde_json::to_string(&recap).unwrap();
        let round_tripped: ContinueActivityRecap = serde_json::from_str(&serialized).unwrap();

        assert_eq!(round_tripped, recap);
        assert_eq!(round_tripped.activity_confidence, ActivityConfidence::High);
        assert_eq!(round_tripped.target_confidence, ActivityConfidence::None);
    }

    #[test]
    fn sanitizer_drops_raw_ids_urls_and_paths_from_public_copy() {
        let recap = ContinueActivityRecap {
            primary_work_summary: Some(
                "Open artifact-secret in /Users/example/file.rs".to_string(),
            ),
            primary_work_label: Some("Writing implementation plan".to_string()),
            primary_where_summary: Some("https://example.com/private".to_string()),
            why_this_target: Some("Selected candidate-secret".to_string()),
            why_no_safe_target: Some("workstream-secret lacks a target".to_string()),
            unfinished_state: Some("See file:///private/tmp/secret.txt".to_string()),
            recent_detours: vec![ActivityDetourSummary {
                surface_title: Some("Search results".to_string()),
                app_name: Some("Browser".to_string()),
                role: ActivityDetourRole::Support,
                activity_label: None,
                reason: "Supported frame-42".to_string(),
                start_ms: None,
                end_ms: None,
                confidence: ActivityEvidenceConfidence::Low,
                evidence_anchor_ids: vec!["frame-42".to_string()],
            }],
            supporting_context: vec![ActivitySupportSummary {
                summary: "Reviewed local documentation".to_string(),
                role: ActivitySupportRole::SourceEvidence,
                confidence: ActivityEvidenceConfidence::Medium,
                evidence_anchor_ids: vec!["artifact-doc".to_string()],
            }],
            evidence_spans: vec![ActivityEvidenceSpan {
                claim_key: "primary_work".to_string(),
                claim_text: "Reviewed local documentation".to_string(),
                anchor_type: ActivityEvidenceAnchorType::Frame,
                anchor_ids: vec!["frame-42".to_string()],
                confidence: ActivityEvidenceConfidence::Medium,
                source: ActivityEvidenceSource::Local,
            }],
            ..ContinueActivityRecap::default()
        }
        .sanitized();

        assert!(recap.primary_work_summary.is_none());
        assert_eq!(
            recap.primary_work_label.as_deref(),
            Some("Writing implementation plan")
        );
        assert!(recap.primary_where_summary.is_none());
        assert!(recap.why_this_target.is_none());
        assert!(recap.why_no_safe_target.is_none());
        assert!(recap.unfinished_state.is_none());
        assert!(recap.recent_detours.is_empty());
        assert_eq!(recap.supporting_context.len(), 1);
        assert_eq!(recap.evidence_spans.len(), 1);
        assert_eq!(recap.evidence_spans[0].anchor_ids, vec!["frame-42"]);
    }
}
