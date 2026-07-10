use serde::{Deserialize, Serialize};

use crate::continuation::{ContinueDecisionResult, ContinueFocusSummary, ContinueReturnTarget};

pub const ISLAND_CONTINUE_STATE_SCHEMA: &str = "smalltalk.island_continue_state.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IslandContinueSource {
    ContinueDecision,
    ContinueDecisionCache,
    NoEvidence,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IslandDisplayState {
    NoLocalMemory,
    LocalMemoryWarming,
    CheckingContinue,
    ContinueReady,
    ThinCurrentWork,
    TargetSuppressed,
    SupportBlocked,
    NeedsRefresh,
    InspectOnly,
    NoClearContinuation,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IslandActionKind {
    RefreshContinue,
    OpenContinueTarget,
    MarkWrongTarget,
    MarkNotUseful,
    InspectEvidence,
    OpenSmalltalk,
    StartLocalMemory,
    CaptureEvidenceNow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IslandAvailableAction {
    pub kind: IslandActionKind,
    pub label: String,
    pub enabled: bool,
    pub decision_id: Option<String>,
}

impl IslandAvailableAction {
    pub fn enabled(kind: IslandActionKind, label: &str, decision_id: Option<String>) -> Self {
        Self {
            kind,
            label: label.to_string(),
            enabled: true,
            decision_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IslandFocusSummary {
    pub title: String,
    pub subtitle: Option<String>,
    pub app_name: Option<String>,
    pub window_title: Option<String>,
    pub openability: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IslandTargetSummary {
    pub title: String,
    pub subtitle: Option<String>,
    pub artifact_kind: Option<String>,
    pub openability: String,
    pub openable: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct IslandFreshness {
    pub evidence_watermark_ms: Option<i64>,
    pub newest_evidence_ms: Option<i64>,
    pub decision_updated_at_ms: Option<i64>,
    pub decision_stale: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct IslandStateContext {
    pub local_memory_running: bool,
    pub has_local_memory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IslandContinueState {
    pub schema: String,
    pub generated_at_ms: i64,
    pub source: IslandContinueSource,
    pub display_state: IslandDisplayState,

    pub decision_id: Option<String>,
    pub decision_cache_hit: bool,
    pub evidence_watermark_ms: Option<i64>,
    pub decision_stale: bool,

    pub current_focus: Option<IslandFocusSummary>,
    pub current_activity: Option<String>,
    pub activity_label: Option<String>,
    pub activity_summary: Option<String>,
    pub activity_where: Option<String>,
    pub activity_state: Option<String>,
    pub activity_confidence_label: Option<String>,
    pub target_confidence_label: Option<String>,
    pub recent_context_summary: Option<String>,
    pub selected_workstream_title: Option<String>,
    pub return_target: Option<IslandTargetSummary>,
    pub resume_work_target: Option<IslandTargetSummary>,

    pub next_action: Option<String>,
    pub confidence_label: Option<String>,
    pub validation_status: Option<String>,
    pub provenance_label: Option<String>,

    pub missing_evidence: Vec<String>,
    pub warnings: Vec<String>,
    pub suppression_reasons: Vec<String>,
    pub available_actions: Vec<IslandAvailableAction>,

    pub inspect_anchor_count: usize,
    pub audit_path: Option<String>,
}

impl IslandContinueState {
    #[allow(dead_code)]
    pub fn no_evidence(freshness: IslandFreshness, context: IslandStateContext) -> Self {
        let display_state = if context.local_memory_running || context.has_local_memory {
            IslandDisplayState::LocalMemoryWarming
        } else {
            IslandDisplayState::NoLocalMemory
        };
        let mut available_actions = vec![IslandAvailableAction::enabled(
            IslandActionKind::OpenSmalltalk,
            "Open Smalltalk",
            None,
        )];
        if context.local_memory_running {
            available_actions.push(IslandAvailableAction::enabled(
                IslandActionKind::CaptureEvidenceNow,
                "Capture evidence now",
                None,
            ));
        } else {
            available_actions.push(IslandAvailableAction::enabled(
                IslandActionKind::StartLocalMemory,
                "Start local memory",
                None,
            ));
        }

        Self {
            schema: ISLAND_CONTINUE_STATE_SCHEMA.to_string(),
            generated_at_ms: freshness.decision_updated_at_ms.unwrap_or_default(),
            source: IslandContinueSource::NoEvidence,
            display_state,
            decision_id: None,
            decision_cache_hit: false,
            evidence_watermark_ms: freshness.evidence_watermark_ms,
            decision_stale: freshness.decision_stale,
            current_focus: None,
            current_activity: None,
            activity_label: None,
            activity_summary: None,
            activity_where: None,
            activity_state: None,
            activity_confidence_label: None,
            target_confidence_label: None,
            recent_context_summary: None,
            selected_workstream_title: None,
            return_target: None,
            resume_work_target: None,
            next_action: Some("Open Smalltalk to inspect local evidence.".to_string()),
            confidence_label: None,
            validation_status: None,
            provenance_label: None,
            missing_evidence: Vec::new(),
            warnings: Vec::new(),
            suppression_reasons: Vec::new(),
            available_actions,
            inspect_anchor_count: 0,
            audit_path: None,
        }
    }

    pub fn refresh_needed(
        freshness: IslandFreshness,
        context: IslandStateContext,
        decision_id: Option<String>,
    ) -> Self {
        Self {
            schema: ISLAND_CONTINUE_STATE_SCHEMA.to_string(),
            generated_at_ms: freshness.decision_updated_at_ms.unwrap_or_default(),
            source: IslandContinueSource::ContinueDecisionCache,
            display_state: IslandDisplayState::NeedsRefresh,
            decision_id,
            decision_cache_hit: true,
            evidence_watermark_ms: freshness.evidence_watermark_ms,
            decision_stale: true,
            current_focus: None,
            current_activity: None,
            activity_label: None,
            activity_summary: None,
            activity_where: None,
            activity_state: None,
            activity_confidence_label: None,
            target_confidence_label: None,
            recent_context_summary: None,
            selected_workstream_title: None,
            return_target: None,
            resume_work_target: None,
            next_action: Some(if context.has_local_memory {
                "Refresh Continue before opening a target.".to_string()
            } else {
                "Open Smalltalk to inspect local evidence.".to_string()
            }),
            confidence_label: None,
            validation_status: Some("needs_refresh".to_string()),
            provenance_label: None,
            missing_evidence: Vec::new(),
            warnings: Vec::new(),
            suppression_reasons: Vec::new(),
            available_actions: vec![
                IslandAvailableAction::enabled(
                    IslandActionKind::RefreshContinue,
                    "Refresh Continue",
                    None,
                ),
                IslandAvailableAction::enabled(
                    IslandActionKind::InspectEvidence,
                    "Inspect evidence",
                    None,
                ),
                IslandAvailableAction::enabled(
                    IslandActionKind::OpenSmalltalk,
                    "Open Smalltalk",
                    None,
                ),
            ],
            inspect_anchor_count: 0,
            audit_path: None,
        }
    }

    pub fn error(generated_at_ms: i64, warning: Option<String>) -> Self {
        Self {
            schema: ISLAND_CONTINUE_STATE_SCHEMA.to_string(),
            generated_at_ms,
            source: IslandContinueSource::Error,
            display_state: IslandDisplayState::Error,
            decision_id: None,
            decision_cache_hit: false,
            evidence_watermark_ms: None,
            decision_stale: false,
            current_focus: None,
            current_activity: None,
            activity_label: None,
            activity_summary: None,
            activity_where: None,
            activity_state: None,
            activity_confidence_label: None,
            target_confidence_label: None,
            recent_context_summary: None,
            selected_workstream_title: None,
            return_target: None,
            resume_work_target: None,
            next_action: Some("Open Smalltalk to inspect local evidence.".to_string()),
            confidence_label: None,
            validation_status: Some("error".to_string()),
            provenance_label: None,
            missing_evidence: Vec::new(),
            warnings: warning.into_iter().collect(),
            suppression_reasons: Vec::new(),
            available_actions: vec![IslandAvailableAction::enabled(
                IslandActionKind::OpenSmalltalk,
                "Open Smalltalk",
                None,
            )],
            inspect_anchor_count: 0,
            audit_path: None,
        }
    }

    pub fn allows_open_continue_target(&self) -> bool {
        self.available_actions.iter().any(|action| {
            action.enabled && matches!(action.kind, IslandActionKind::OpenContinueTarget)
        })
    }
}

pub fn island_state_from_continue_decision(
    decision: &ContinueDecisionResult,
    freshness: IslandFreshness,
    context: IslandStateContext,
) -> IslandContinueState {
    let current_focus = decision.current_focus.as_ref().and_then(focus_summary);
    let return_target = decision.return_target.as_ref().and_then(target_summary);
    let resume_work_target = decision
        .resume_work_target
        .as_ref()
        .and_then(target_summary);
    let inspect_anchor_count = decision.evidence_anchors.frame_ids.len()
        + decision.evidence_anchors.action_ids.len()
        + decision.evidence_anchors.episode_ids.len()
        + decision.evidence_anchors.artifact_ids.len();
    let suppression_reasons = suppression_reasons(decision);
    let support_blocked = support_branch_blocked(decision);
    let target_suppressed = !suppression_reasons.is_empty();
    let thin = decision_is_thin(decision);
    let has_openable_target = return_target
        .as_ref()
        .or(resume_work_target.as_ref())
        .is_some_and(|target| target.openable);
    let has_any_target = return_target.is_some() || resume_work_target.is_some();
    let validation_rejected = validation_rejected(&decision.validation_status);
    let no_clear_output = decision.continue_output_mode == "no_clear_continuation";
    let decision_stale = freshness.decision_stale
        || freshness
            .newest_evidence_ms
            .zip(freshness.decision_updated_at_ms)
            .is_some_and(|(newest, decision_at)| newest > decision_at);
    let recap = &decision.activity_recap;
    let recent_context_summary = recap
        .recent_detours
        .first()
        .and_then(|detour| safe_text(Some(&detour.reason)))
        .or_else(|| {
            recap
                .supporting_context
                .first()
                .and_then(|support| safe_text(Some(&support.summary)))
        });
    let recap_has_useful_copy = safe_text(recap.primary_work_summary.as_deref()).is_some()
        || safe_text(recap.primary_work_label.as_deref()).is_some()
        || safe_text(recap.primary_where_summary.as_deref()).is_some()
        || safe_text(recap.last_meaningful_state.as_deref()).is_some()
        || safe_text(recap.unfinished_state.as_deref()).is_some()
        || recent_context_summary.is_some();
    let missing_evidence = merge_safe_notes(&recap.missing_evidence, &decision.missing_evidence);
    let warnings = merge_safe_notes(&recap.warnings, &decision.warnings);

    let can_open = !decision_stale
        && !target_suppressed
        && !support_blocked
        && !thin
        && !validation_rejected
        && !no_clear_output
        && has_openable_target
        && !decision.decision_id.trim().is_empty();

    let display_state = if decision_stale {
        IslandDisplayState::NeedsRefresh
    } else if support_blocked {
        IslandDisplayState::SupportBlocked
    } else if target_suppressed {
        IslandDisplayState::TargetSuppressed
    } else if no_clear_output {
        IslandDisplayState::NoClearContinuation
    } else if can_open {
        IslandDisplayState::ContinueReady
    } else if decision.current_focus.is_some() && !has_any_target {
        if decision.active_current_work_unresolved.is_some() || thin {
            IslandDisplayState::ThinCurrentWork
        } else {
            IslandDisplayState::NoClearContinuation
        }
    } else if thin {
        IslandDisplayState::ThinCurrentWork
    } else if inspect_anchor_count > 0 {
        IslandDisplayState::InspectOnly
    } else if context.local_memory_running || context.has_local_memory {
        IslandDisplayState::LocalMemoryWarming
    } else {
        IslandDisplayState::NoClearContinuation
    };

    let mut available_actions = Vec::new();
    if decision_stale {
        available_actions.push(IslandAvailableAction::enabled(
            IslandActionKind::RefreshContinue,
            "Refresh Continue",
            None,
        ));
    }
    if can_open {
        available_actions.push(IslandAvailableAction::enabled(
            IslandActionKind::OpenContinueTarget,
            "Open Continue target",
            Some(decision.decision_id.clone()),
        ));
    }
    if has_any_target && !decision.decision_id.trim().is_empty() {
        available_actions.push(IslandAvailableAction::enabled(
            IslandActionKind::MarkWrongTarget,
            "Wrong target",
            Some(decision.decision_id.clone()),
        ));
        available_actions.push(IslandAvailableAction::enabled(
            IslandActionKind::MarkNotUseful,
            "Not useful",
            Some(decision.decision_id.clone()),
        ));
    }
    if inspect_anchor_count > 0
        || thin
        || !has_openable_target
        || support_blocked
        || target_suppressed
        || decision.active_current_work_unresolved.is_some()
    {
        available_actions.push(IslandAvailableAction::enabled(
            IslandActionKind::InspectEvidence,
            "Inspect evidence",
            None,
        ));
    }
    available_actions.push(IslandAvailableAction::enabled(
        IslandActionKind::OpenSmalltalk,
        "Open Smalltalk",
        None,
    ));
    if matches!(
        display_state,
        IslandDisplayState::ThinCurrentWork | IslandDisplayState::NoClearContinuation
    ) {
        available_actions.push(IslandAvailableAction::enabled(
            IslandActionKind::CaptureEvidenceNow,
            "Capture evidence now",
            None,
        ));
    }

    IslandContinueState {
        schema: ISLAND_CONTINUE_STATE_SCHEMA.to_string(),
        generated_at_ms: freshness.decision_updated_at_ms.unwrap_or_default(),
        source: if decision.cache_hit {
            IslandContinueSource::ContinueDecisionCache
        } else {
            IslandContinueSource::ContinueDecision
        },
        display_state,
        decision_id: Some(decision.decision_id.clone()).filter(|id| !id.trim().is_empty()),
        decision_cache_hit: decision.cache_hit,
        evidence_watermark_ms: freshness.evidence_watermark_ms,
        decision_stale,
        current_focus,
        current_activity: safe_text(decision.current_activity.as_deref()),
        activity_label: safe_text(recap.primary_work_label.as_deref()),
        activity_summary: safe_text(recap.primary_work_summary.as_deref()),
        activity_where: safe_text(recap.primary_where_summary.as_deref()),
        activity_state: safe_text(
            recap
                .last_meaningful_state
                .as_deref()
                .or(recap.unfinished_state.as_deref())
                .or_else(|| activity_state_label(recap.current_state)),
        ),
        activity_confidence_label: recap_has_useful_copy
            .then(|| activity_confidence_label(recap.activity_confidence).to_string()),
        target_confidence_label: recap_has_useful_copy
            .then(|| activity_confidence_label(recap.target_confidence).to_string()),
        recent_context_summary,
        selected_workstream_title: decision
            .selected_workstream
            .as_ref()
            .and_then(|workstream| safe_text(workstream.title_candidate.as_deref())),
        return_target,
        resume_work_target,
        next_action: safe_text(
            recap.next_action_summary.as_deref().or(decision
                .next_action
                .as_deref()
                .or(Some(decision.handoff.next_action.as_str()))),
        ),
        confidence_label: safe_text(Some(&decision.confidence_label)),
        validation_status: safe_text(Some(&decision.validation_status)),
        provenance_label: safe_text(Some(&decision.source)),
        missing_evidence,
        warnings,
        suppression_reasons,
        available_actions,
        inspect_anchor_count,
        audit_path: None,
    }
}

fn activity_state_label(
    state: crate::continuation::activity_recap::ActivityCurrentState,
) -> Option<&'static str> {
    use crate::continuation::activity_recap::ActivityCurrentState;
    match state {
        ActivityCurrentState::ActivelyWorking => Some("Actively working"),
        ActivityCurrentState::RecentlyDetoured => Some("Recently detoured"),
        ActivityCurrentState::PausedAfterProgress => Some("Paused after progress"),
        ActivityCurrentState::Blocked => Some("Blocked"),
        ActivityCurrentState::CompleteOrIdle => Some("Complete or idle"),
        ActivityCurrentState::Unclear => None,
    }
}

fn activity_confidence_label(
    confidence: crate::continuation::activity_recap::ActivityConfidence,
) -> &'static str {
    use crate::continuation::activity_recap::ActivityConfidence;
    match confidence {
        ActivityConfidence::High => "high",
        ActivityConfidence::Medium => "medium",
        ActivityConfidence::Low => "low",
        ActivityConfidence::None => "none",
    }
}

fn focus_summary(focus: &ContinueFocusSummary) -> Option<IslandFocusSummary> {
    let title = first_safe_text(&[
        focus.display_title.as_deref(),
        focus.title.as_deref(),
        focus.window_title.as_deref(),
        focus.app_name.as_deref(),
    ])?;
    let subtitle =
        safe_text(focus.app_name.as_deref()).or_else(|| safe_text(focus.artifact_kind.as_deref()));
    Some(IslandFocusSummary {
        title,
        subtitle,
        app_name: safe_text(focus.app_name.as_deref()),
        window_title: safe_text(focus.window_title.as_deref()),
        openability: safe_text(focus.openability.as_deref()),
    })
}

fn target_summary(target: &ContinueReturnTarget) -> Option<IslandTargetSummary> {
    let artifact_kind = safe_text(target.artifact_kind.as_deref());
    let title = safe_text(target.title.as_deref())
        .or_else(|| artifact_kind.clone())
        .unwrap_or_else(|| "Continue target".to_string());
    let openability = safe_text(Some(&target.openability)).unwrap_or_else(|| "unknown".to_string());
    let openable = openability == "openable";
    Some(IslandTargetSummary {
        title,
        subtitle: artifact_kind.clone(),
        artifact_kind,
        openability,
        openable,
    })
}

fn first_safe_text(values: &[Option<&str>]) -> Option<String> {
    values.iter().find_map(|value| safe_text(*value))
}

fn decision_is_thin(decision: &ContinueDecisionResult) -> bool {
    decision.confidence < 0.55
        || decision.confidence_label.eq_ignore_ascii_case("thin")
        || decision.confidence_label.eq_ignore_ascii_case("low")
        || decision.validation_status.contains("thin")
        || decision.validation_status.contains("no_clear")
        || decision.validation_status.contains("no_candidates")
        || !decision.validation_failures.is_empty()
        || decision.p0_quality_signals.thin_mode_truthful
        || decision
            .active_current_work_unresolved
            .as_ref()
            .is_some_and(|work| !work.has_openable_target)
}

fn validation_rejected(status: &str) -> bool {
    status.contains("rejected")
        || status.contains("blocked")
        || status.contains("invalid")
        || status.contains("suppressed")
}

fn support_branch_blocked(decision: &ContinueDecisionResult) -> bool {
    !decision.branch_validation_failures.is_empty()
        || !decision.excluded_branch_candidate_ids.is_empty()
        || decision
            .warnings
            .iter()
            .any(|warning| warning.contains("branch_support_not_default_return_target"))
}

fn suppression_reasons(decision: &ContinueDecisionResult) -> Vec<String> {
    let mut reasons = Vec::new();
    if decision.p0_quality_signals.stale_target_suppressed {
        reasons.push("stale_target_suppressed".to_string());
    }
    if decision
        .p0_quality_signals
        .selected_target_older_than_current_focus
    {
        reasons.push("selected_target_older_than_current_focus".to_string());
    }
    if let Some(audit) = decision
        .continue_dossier
        .as_ref()
        .and_then(|dossier| dossier.stale_target_suppression.as_ref())
        .filter(|audit| audit.suppressed)
    {
        reasons.push(format!("stale_target:{}", audit.reason));
    }
    if decision.feedback_suppressed_candidate_count > 0 {
        reasons.push("feedback_suppressed_candidate".to_string());
    }
    if decision.feedback_score_capped_candidate_count > 0 && decision.return_target.is_none() {
        reasons.push("feedback_score_capped_target".to_string());
    }
    if let Some(work) = decision
        .active_current_work_unresolved
        .as_ref()
        .filter(|work| !work.has_openable_target)
    {
        reasons.push(format!("active_current_work:{}", work.unresolved_reason));
    }
    for failure in &decision.validation_failures {
        if let Some(value) = safe_code_or_note(failure) {
            reasons.push(format!("validation:{value}"));
        }
    }
    reasons.sort();
    reasons.dedup();
    reasons
}

fn safe_text(value: Option<&str>) -> Option<String> {
    let text = value?
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    if text.is_empty() || looks_like_raw_locator(&text) || looks_like_internal_text(&text) {
        return None;
    }
    Some(text.chars().take(160).collect())
}

fn merge_safe_notes(primary: &[String], secondary: &[String]) -> Vec<String> {
    let mut notes = Vec::new();
    for value in primary.iter().chain(secondary) {
        if let Some(value) = safe_code_or_note(value) {
            if !notes.contains(&value) {
                notes.push(value);
            }
        }
    }
    notes
}

fn safe_code_or_note(value: &str) -> Option<String> {
    let text = safe_text(Some(value))?;
    Some(text.chars().take(180).collect())
}

fn looks_like_raw_locator(value: &str) -> bool {
    let lower = value.to_lowercase();
    lower.contains("://")
        || lower.contains("www.")
        || lower.starts_with("file:")
        || lower.starts_with("/users/")
        || lower.starts_with("/private/")
        || lower.starts_with("~/")
        || lower.contains("\\")
        || lower.contains("/users/")
        || lower.contains("/private/")
        || lower.split_whitespace().any(|token| {
            token
                .trim_matches(|ch: char| matches!(ch, '(' | ')' | '[' | ']' | ',' | '.'))
                .starts_with('/')
        })
}

fn looks_like_internal_text(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("continue-candidate-")
        || lower.contains("continue-decision-")
        || lower.contains("candidate-")
        || lower.contains("workstream-")
        || lower.contains("artifact-")
        || lower.contains("task-action-")
        || lower.contains("action-")
        || lower.contains("episode-")
        || lower.contains("open-loop-")
        || lower.contains("frame-fallback")
        || lower.contains("frame_fallback")
        || lower.contains("frame_id")
        || lower.contains("frame id")
        || lower.contains("semantic moment")
        || lower.contains("open loop")
        || lower.contains("sqlite")
        || contains_adjacent_words(&lower, "resume", "query")
        || contains_adjacent_words(&lower, "cloud", "resume")
        || lower.contains("scorer")
}

fn contains_adjacent_words(value: &str, first: &str, second: &str) -> bool {
    value
        .split_whitespace()
        .zip(value.split_whitespace().skip(1))
        .any(|(left, right)| left == first && right == second)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuation::enrichment::WeakSurfaceEnrichmentDiagnostics;
    use crate::continuation::{
        ContinueEvidenceAnchors, ContinueHandoff, ContinueSelectedWorkstream, P0QualitySignals,
    };

    fn base_decision() -> ContinueDecisionResult {
        ContinueDecisionResult {
            decision_id: "continue-test".to_string(),
            mode: "normal".to_string(),
            cache_hit: false,
            cache_bypass_reasons: Vec::new(),
            source: "local_scorer".to_string(),
            model: None,
            response_id: None,
            current_focus: Some(ContinueFocusSummary {
                frame_id: "frame-current".to_string(),
                artifact_id: None,
                artifact_kind: Some("codex_thread".to_string()),
                domain: None,
                app_name: Some("Codex".to_string()),
                window_title: Some("Smalltalk P4".to_string()),
                title: Some("Smalltalk P4".to_string()),
                display_title: Some("Smalltalk P4".to_string()),
                browser_url: None,
                document_path: None,
                activity_state: None,
                task_state: None,
                evidence_quality: Some("strong".to_string()),
                identity_confidence: Some(0.9),
                snapshot_id: None,
                missing_fields: Vec::new(),
                openability: Some("inspect_only".to_string()),
                captured_at_ms: 2_000,
            }),
            active_current_work_unresolved: None,
            p0_quality_signals: P0QualitySignals {
                return_target_openable: true,
                ..P0QualitySignals::default()
            },
            current_activity: Some("Implementing island Continue state".to_string()),
            selected_workstream: Some(ContinueSelectedWorkstream {
                workstream_id: "workstream-test".to_string(),
                state: "active".to_string(),
                title_candidate: Some("P4 Island Contract".to_string()),
                primary_artifact_id: Some("artifact-target".to_string()),
                last_active_timestamp_ms: 1_900,
                unresolved_signal: Some("implementation".to_string()),
            }),
            selected_candidate_id: Some("candidate-test".to_string()),
            return_target: Some(ContinueReturnTarget {
                artifact_id: Some("artifact-target".to_string()),
                artifact_kind: Some("code_file".to_string()),
                title: Some("session_island.rs".to_string()),
                browser_url: Some("https://example.com/raw-should-not-leak".to_string()),
                document_path: Some("/Users/example/raw-should-not-leak.rs".to_string()),
                openability: "openable".to_string(),
                fallback_frame_id: Some("frame-target".to_string()),
            }),
            resume_work_target: None,
            candidate_kind: Some("primary".to_string()),
            last_meaningful_action: None,
            unresolved_state: None,
            next_action: Some("Continue from the island contract implementation.".to_string()),
            confidence: 0.82,
            confidence_label: "High".to_string(),
            evidence_anchors: ContinueEvidenceAnchors {
                frame_ids: vec!["frame-current".to_string()],
                action_ids: vec!["action-current".to_string()],
                episode_ids: Vec::new(),
                artifact_ids: vec!["artifact-target".to_string()],
            },
            missing_evidence: Vec::new(),
            warnings: Vec::new(),
            validation_failures: Vec::new(),
            handoff: ContinueHandoff {
                headline: "Ready to continue".to_string(),
                return_line: "Return to session_island.rs.".to_string(),
                current_focus_line: "Current focus is Codex.".to_string(),
                last_state_line: "Editing island state contract.".to_string(),
                next_action: "Continue from the island contract implementation.".to_string(),
                why_this: Vec::new(),
                missing_evidence_line: None,
                confidence_label: "High".to_string(),
                user_visible_uncertainty: None,
            },
            support_evidence: Vec::new(),
            alternatives: Vec::new(),
            generated_candidates: 1,
            validation_status: "validated".to_string(),
            feedback_policy_version: "test".to_string(),
            feedback_watermark_ms: None,
            open_watermark_ms: None,
            feedback_suppressed_candidate_count: 0,
            feedback_score_capped_candidate_count: 0,
            eligible_candidate_count_after_feedback_gate: 1,
            model_candidate_count_before_feedback_filter: 1,
            model_candidate_count_after_feedback_filter: 1,
            selectable_candidate_count_before_branch_filter: 1,
            selectable_candidate_count_after_branch_filter: 1,
            excluded_branch_candidate_ids: Vec::new(),
            support_evidence_count: 0,
            branch_validation_failures: Vec::new(),
            continue_output_mode: "normal".to_string(),
            evidence_watermark_hash: "watermark-test".to_string(),
            latest_boundary_revision: None,
            current_surface_resolution: None,
            evidence_freshness_ledger: None,
            continue_dossier: None,
            memory_retrieval: None,
            observe_before_decide: None,
            weak_surface_enrichment: WeakSurfaceEnrichmentDiagnostics::default(),
            app_activity: None,
            activity_summary: None,
            activity_recap: crate::continuation::ContinueActivityRecap::default(),
            activity_recap_watermark_hash: String::new(),
            activity_recap_synthesis_audit: Default::default(),
            activity_recap_proof: Default::default(),
            quality_gate: None,
            evidence_pack_v2_used: false,
            micro_inference_requested: false,
            micro_inference_attempted: false,
            micro_inference_result_kind: None,
            continue_output_path: Some("/Users/example/continue_outputs/raw-path".to_string()),
            audit_inference_events: Vec::new(),
        }
    }

    fn test_freshness() -> IslandFreshness {
        IslandFreshness {
            evidence_watermark_ms: Some(2_000),
            newest_evidence_ms: Some(2_000),
            decision_updated_at_ms: Some(2_100),
            decision_stale: false,
        }
    }

    fn mapped(decision: &ContinueDecisionResult) -> IslandContinueState {
        island_state_from_continue_decision(
            decision,
            test_freshness(),
            IslandStateContext {
                local_memory_running: true,
                has_local_memory: true,
            },
        )
    }

    #[test]
    fn island_state_continue_ready_has_open_action_with_decision_id() {
        let decision = base_decision();
        let state = mapped(&decision);

        assert_eq!(state.display_state, IslandDisplayState::ContinueReady);
        assert!(state.allows_open_continue_target());
        assert!(state.available_actions.iter().any(|action| {
            action.kind == IslandActionKind::OpenContinueTarget
                && action.decision_id.as_deref() == Some("continue-test")
        }));
        assert!(state.activity_summary.is_none());
        assert!(state.activity_confidence_label.is_none());
        assert!(state.target_confidence_label.is_none());
    }

    #[test]
    fn island_state_thin_current_work_has_no_open_continue_action() {
        let mut decision = base_decision();
        decision.confidence = 0.42;
        decision.confidence_label = "Thin".to_string();
        decision.validation_status = "thin_evidence".to_string();
        decision.missing_evidence = vec!["missing_fresh_heavy_frame_for_current_focus".to_string()];
        decision.activity_recap.primary_work_summary =
            Some("Writing the P5 activity-memory card".to_string());
        decision.activity_recap.activity_confidence =
            crate::continuation::activity_recap::ActivityConfidence::Medium;
        decision.activity_recap.target_confidence =
            crate::continuation::activity_recap::ActivityConfidence::Low;

        let state = mapped(&decision);

        assert_eq!(state.display_state, IslandDisplayState::ThinCurrentWork);
        assert!(!state.allows_open_continue_target());
        assert!(state
            .available_actions
            .iter()
            .any(|action| action.kind == IslandActionKind::InspectEvidence));
        assert_eq!(
            state.activity_summary.as_deref(),
            Some("Writing the P5 activity-memory card")
        );
        assert_eq!(state.activity_confidence_label.as_deref(), Some("medium"));
        assert_eq!(state.target_confidence_label.as_deref(), Some("low"));
    }

    #[test]
    fn island_state_suppressed_target_has_no_open_continue_action() {
        let mut decision = base_decision();
        decision.p0_quality_signals.stale_target_suppressed = true;
        decision
            .p0_quality_signals
            .selected_target_older_than_current_focus = true;

        let state = mapped(&decision);

        assert_eq!(state.display_state, IslandDisplayState::TargetSuppressed);
        assert!(!state.allows_open_continue_target());
        assert!(state
            .suppression_reasons
            .contains(&"stale_target_suppressed".to_string()));
    }

    #[test]
    fn island_state_p1_hard_suppressed_has_no_open_continue_action() {
        let mut decision = base_decision();
        decision.feedback_suppressed_candidate_count = 1;
        decision.validation_status = "suppressed_by_feedback".to_string();

        let state = mapped(&decision);

        assert_eq!(state.display_state, IslandDisplayState::TargetSuppressed);
        assert!(!state.allows_open_continue_target());
        assert!(state
            .suppression_reasons
            .contains(&"feedback_suppressed_candidate".to_string()));
    }

    #[test]
    fn island_state_support_blocked_has_no_open_continue_action_unless_origin_valid() {
        let mut decision = base_decision();
        decision.branch_validation_failures =
            vec!["branch_support_not_default_return_target".to_string()];

        let blocked = mapped(&decision);

        assert_eq!(blocked.display_state, IslandDisplayState::SupportBlocked);
        assert!(!blocked.allows_open_continue_target());

        decision.branch_validation_failures.clear();
        decision.warnings.clear();
        let valid_origin = mapped(&decision);
        assert_eq!(
            valid_origin.display_state,
            IslandDisplayState::ContinueReady
        );
        assert!(valid_origin.allows_open_continue_target());
    }

    #[test]
    fn island_state_needs_refresh_prefers_refresh_action() {
        let decision = base_decision();
        let state = island_state_from_continue_decision(
            &decision,
            IslandFreshness {
                evidence_watermark_ms: Some(2_500),
                newest_evidence_ms: Some(2_500),
                decision_updated_at_ms: Some(2_100),
                decision_stale: true,
            },
            IslandStateContext::default(),
        );

        assert_eq!(state.display_state, IslandDisplayState::NeedsRefresh);
        assert!(!state.allows_open_continue_target());
        assert_eq!(
            state.available_actions.first().map(|action| &action.kind),
            Some(&IslandActionKind::RefreshContinue)
        );
    }

    #[test]
    fn island_state_no_clear_continuation_has_no_open_continue_action() {
        let mut decision = base_decision();
        decision.return_target = None;
        decision.resume_work_target = None;
        decision.confidence = 0.7;
        decision.confidence_label = "Medium".to_string();
        decision.validation_status = "validated".to_string();

        let state = mapped(&decision);

        assert_eq!(state.display_state, IslandDisplayState::NoClearContinuation);
        assert!(!state.allows_open_continue_target());
    }

    #[test]
    fn island_no_clear_output_never_reopens_a_leaked_target() {
        let mut decision = base_decision();
        decision.continue_output_mode = "no_clear_continuation".to_string();

        let state = mapped(&decision);

        assert_eq!(state.display_state, IslandDisplayState::NoClearContinuation);
        assert!(!state.allows_open_continue_target());
        assert!(state
            .available_actions
            .iter()
            .all(|action| action.kind != IslandActionKind::OpenContinueTarget));
    }

    #[test]
    fn island_state_p3_weak_surface_inspect_only_has_no_open_continue_action() {
        let mut decision = base_decision();
        decision.current_focus = None;
        decision.return_target = None;
        decision.resume_work_target = None;
        decision.confidence = 0.74;
        decision.confidence_label = "Medium".to_string();
        decision.validation_status = "validated".to_string();

        let state = mapped(&decision);

        assert_eq!(state.display_state, IslandDisplayState::InspectOnly);
        assert!(!state.allows_open_continue_target());
        assert!(state
            .available_actions
            .iter()
            .any(|action| action.kind == IslandActionKind::InspectEvidence));
    }

    #[test]
    fn island_state_error_has_no_open_continue_action() {
        let state = IslandContinueState::error(2_000, Some("test_error".to_string()));

        assert_eq!(state.display_state, IslandDisplayState::Error);
        assert!(!state.allows_open_continue_target());
        assert!(state
            .available_actions
            .iter()
            .all(|action| action.kind != IslandActionKind::OpenContinueTarget));
    }

    #[test]
    fn island_state_redacts_raw_url_and_path() {
        let mut decision = base_decision();
        decision.return_target.as_mut().unwrap().title =
            Some("https://example.com/raw-target".to_string());
        decision.current_focus.as_mut().unwrap().display_title =
            Some("/Users/example/private-project/file.rs".to_string());
        decision.current_focus.as_mut().unwrap().title = Some("Safe focus title".to_string());

        let state = mapped(&decision);
        let payload = serde_json::to_string(&state).unwrap();

        assert!(!payload.contains("https://example.com"));
        assert!(!payload.contains("/Users/example"));
        assert!(!payload.contains("raw-path"));
        assert_eq!(
            state
                .return_target
                .as_ref()
                .map(|target| target.title.as_str()),
            Some("code_file")
        );
        assert_eq!(
            state
                .current_focus
                .as_ref()
                .map(|focus| focus.title.as_str()),
            Some("Safe focus title")
        );
    }

    #[test]
    fn island_compact_recap_matches_main_decision_without_exposing_proof_ids() {
        let mut decision = base_decision();
        decision.activity_recap.primary_work_summary =
            Some("Integrating P5 into the Continue lifecycle".to_string());
        decision.activity_recap.primary_work_label = Some("Integrating P5".to_string());
        decision.activity_recap.primary_where_summary = Some("Smalltalk codebase".to_string());
        decision.activity_recap.last_meaningful_state =
            Some("The recap pipeline was connected to the decision.".to_string());
        decision.activity_recap.activity_confidence =
            crate::continuation::activity_recap::ActivityConfidence::High;
        decision.activity_recap.target_confidence =
            crate::continuation::activity_recap::ActivityConfidence::Medium;
        decision.activity_recap.next_action_summary =
            Some("Verify the Continue card and island together.".to_string());
        decision.activity_recap.missing_evidence =
            vec!["The exact active file was not visible.".to_string()];
        decision.activity_recap.recent_detours.push(
            crate::continuation::activity_recap::ActivityDetourSummary {
                surface_title: Some("Finder".to_string()),
                app_name: Some("Finder".to_string()),
                role: crate::continuation::activity_recap::ActivityDetourRole::Detour,
                activity_label: Some("File browsing".to_string()),
                reason: "Finder was a brief detour from the primary work.".to_string(),
                start_ms: Some(1_000),
                end_ms: Some(1_100),
                confidence: crate::continuation::activity_recap::ActivityEvidenceConfidence::Medium,
                evidence_anchor_ids: vec!["frame-secret-anchor".to_string()],
            },
        );

        let state = mapped(&decision);
        let payload = serde_json::to_string(&state).unwrap();

        assert_eq!(state.display_state, IslandDisplayState::ContinueReady);
        assert!(state.allows_open_continue_target());
        assert_eq!(
            state.activity_summary.as_deref(),
            decision.activity_recap.primary_work_summary.as_deref()
        );
        assert_eq!(state.activity_where.as_deref(), Some("Smalltalk codebase"));
        assert_eq!(state.activity_label.as_deref(), Some("Integrating P5"));
        assert_eq!(
            state.activity_state.as_deref(),
            Some("The recap pipeline was connected to the decision.")
        );
        assert_eq!(state.activity_confidence_label.as_deref(), Some("high"));
        assert_eq!(state.target_confidence_label.as_deref(), Some("medium"));
        assert_eq!(
            state.next_action.as_deref(),
            Some("Verify the Continue card and island together.")
        );
        assert_eq!(
            state.missing_evidence.first().map(String::as_str),
            Some("The exact active file was not visible.")
        );
        assert_eq!(
            state.recent_context_summary.as_deref(),
            Some("Finder was a brief detour from the primary work.")
        );
        assert!(!payload.contains("activity_recap"));
        assert!(!payload.contains("frame-secret-anchor"));
        assert!(!payload.contains("/Users/"));
        assert!(!payload.contains("://"));
    }

    #[test]
    fn island_keeps_useful_activity_when_there_is_no_safe_target() {
        let mut decision = base_decision();
        decision.return_target = None;
        decision.resume_work_target = None;
        decision.activity_recap.primary_work_summary =
            Some("Planning the P5 activity-memory UI".to_string());
        decision.activity_recap.primary_work_label = Some("Planning P5 UI".to_string());
        decision.activity_recap.primary_where_summary = Some("Smalltalk project".to_string());
        decision.activity_recap.activity_confidence =
            crate::continuation::activity_recap::ActivityConfidence::High;
        decision.activity_recap.target_confidence =
            crate::continuation::activity_recap::ActivityConfidence::None;
        decision.activity_recap.why_no_safe_target =
            Some("The exact thread was not visible.".to_string());

        let state = mapped(&decision);

        assert_eq!(state.display_state, IslandDisplayState::NoClearContinuation);
        assert!(!state.allows_open_continue_target());
        assert!(state
            .available_actions
            .iter()
            .any(|action| action.kind == IslandActionKind::InspectEvidence));
        assert_eq!(state.activity_label.as_deref(), Some("Planning P5 UI"));
        assert_eq!(state.activity_confidence_label.as_deref(), Some("high"));
        assert_eq!(state.target_confidence_label.as_deref(), Some("none"));
    }

    #[test]
    fn island_filters_internal_looking_activity_copy() {
        let mut decision = base_decision();
        decision.activity_recap.primary_work_summary =
            Some("Review candidate-secret before opening".to_string());
        decision.activity_recap.primary_work_label = Some("artifact-secret".to_string());
        decision.activity_recap.primary_where_summary = Some("workstream-secret".to_string());
        decision.activity_recap.last_meaningful_state =
            Some("frame-fallback remained visible".to_string());

        let state = mapped(&decision);

        assert!(state.activity_summary.is_none());
        assert!(state.activity_label.is_none());
        assert!(state.activity_where.is_none());
        assert!(state.activity_state.is_none());
        assert!(state.activity_confidence_label.is_none());
        assert!(state.target_confidence_label.is_none());
    }
}
