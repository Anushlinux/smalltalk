use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::capture::{CaptureState, CaptureStatus};

use super::{
    continue_freshness_from_island_state, contract::authoritative_task_truth_answer,
    decision_evidence_updated_at_ms, headline_from_island_state, island_continue_decision_request,
    IslandContinueState, IslandDisplayState, IslandFreshness, IslandStateContext,
    RememberedContinueIslandState, LAST_CONTINUE_ISLAND_STATE,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IslandContinueReason {
    InitialRender,
    UserOpenedIsland,
    UserPressedContinue,
    EvidenceChanged,
    TimerRefresh,
    MainCardDecisionUpdated,
}

impl IslandContinueReason {
    fn as_str(&self) -> &'static str {
        match self {
            Self::InitialRender => "initial_render",
            Self::UserOpenedIsland => "user_opened_island",
            Self::UserPressedContinue => "user_pressed_continue",
            Self::EvidenceChanged => "evidence_changed",
            Self::TimerRefresh => "timer_refresh",
            Self::MainCardDecisionUpdated => "main_card_decision_updated",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IslandContinueStateInput {
    pub reason: IslandContinueReason,
    pub existing_decision_id: Option<String>,
    pub allow_refresh: bool,
    pub force_refresh: bool,
    pub source: Option<String>,
}

impl IslandContinueStateInput {
    pub fn for_user_continue(existing_decision_id: Option<String>) -> Self {
        Self {
            reason: IslandContinueReason::UserPressedContinue,
            existing_decision_id,
            allow_refresh: true,
            force_refresh: true,
            source: Some("native_island".to_string()),
        }
    }
}

pub struct IslandContinueGatewayResult {
    pub state: IslandContinueState,
    pub decision: Option<crate::continuation::ContinueDecisionResult>,
}

pub fn get_island_continue_state(
    app: AppHandle,
    state: State<CaptureState>,
    input: IslandContinueStateInput,
) -> Result<IslandContinueState, String> {
    let status = crate::capture::capture_status(app.clone(), state)?;
    let result = get_island_continue_state_for_status(app, status, input)?;
    Ok(result.state)
}

pub fn get_island_continue_state_for_status(
    app: AppHandle,
    status: CaptureStatus,
    input: IslandContinueStateInput,
) -> Result<IslandContinueGatewayResult, String> {
    let context = context_from_status(&status);
    let newest_evidence_ms = newest_status_evidence_ms(&status);
    let now_ms = super::now_millis();
    let freshness = IslandFreshness {
        evidence_watermark_ms: newest_evidence_ms,
        newest_evidence_ms,
        decision_updated_at_ms: Some(now_ms),
        decision_stale: false,
    };

    if !context.has_local_memory {
        let mut state = IslandContinueState::no_evidence(freshness, context);
        annotate_gateway_state(&mut state, &input);
        emit_gateway_state(&app, &state);
        return Ok(IslandContinueGatewayResult {
            state,
            decision: None,
        });
    }

    if !input.force_refresh {
        let feedback_or_open_watermark_ms =
            crate::capture::latest_continue_feedback_or_open_watermark_ms(&app)
                .ok()
                .flatten();
        if let Some(state) = remembered_fresh_state(&status, &input, feedback_or_open_watermark_ms)
        {
            emit_gateway_state(&app, &state);
            return Ok(IslandContinueGatewayResult {
                state,
                decision: None,
            });
        }

        if !input.allow_refresh {
            let mut state = IslandContinueState::refresh_needed(
                IslandFreshness {
                    decision_stale: true,
                    ..freshness
                },
                context,
                input.existing_decision_id.clone(),
            );
            annotate_gateway_state(&mut state, &input);
            emit_gateway_state(&app, &state);
            return Ok(IslandContinueGatewayResult {
                state,
                decision: None,
            });
        }
    }

    let mut request = continue_decision_request_for_input(&input);
    request.session_id = status
        .active_session
        .as_ref()
        .or(status.latest_session.as_ref())
        .map(|session| session.id.clone());
    let decision = crate::capture::get_continue_decision(app.clone(), Some(request))?;
    let mut state = island_state_from_decision_status(&decision, &status, false);
    annotate_gateway_state(&mut state, &input);
    state.audit_path = decision.continue_output_path.clone();
    if let Some((mut retained, rejection_reasons)) =
        rejected_gateway_adoption(&decision, &state, &status, &input)
    {
        for reason in &rejection_reasons {
            retained.warnings.push(format!("result_adoption:{reason}"));
        }
        retained.warnings.sort();
        retained.warnings.dedup();
        annotate_gateway_state(&mut retained, &input);
        super::write_island_continue_audit(
            retained.audit_path.as_deref(),
            &retained,
            input.reason.as_str(),
            input.source.as_deref().unwrap_or("native_island"),
            false,
            retained.allows_open_continue_target(),
            Some(&rejection_reasons.join(",")),
        );
        emit_gateway_state(&app, &retained);
        return Ok(IslandContinueGatewayResult {
            state: retained,
            decision: None,
        });
    }
    super::write_island_continue_audit(
        state.audit_path.as_deref(),
        &state,
        input.reason.as_str(),
        input.source.as_deref().unwrap_or("native_island"),
        false,
        state.allows_open_continue_target(),
        None,
    );
    let feedback_or_open_watermark_ms =
        crate::capture::latest_continue_feedback_or_open_watermark_ms(&app)
            .ok()
            .flatten();
    remember_gateway_decision(&decision, &state, &status, feedback_or_open_watermark_ms);
    emit_gateway_state(&app, &state);

    Ok(IslandContinueGatewayResult {
        state,
        decision: Some(decision),
    })
}

fn continue_decision_request_for_input(
    input: &IslandContinueStateInput,
) -> crate::continuation::ContinueDecisionRequest {
    let mut request = island_continue_decision_request();
    let user_pressed_continue = matches!(&input.reason, IslandContinueReason::UserPressedContinue);
    request.audit_output_enabled = Some(false);
    request.audit_mode = Some(if user_pressed_continue {
        crate::continuation::ContinueAuditMode::MftiReview
    } else {
        crate::continuation::ContinueAuditMode::None
    });
    request.request_trigger = Some(if user_pressed_continue {
        "manual".to_string()
    } else {
        "island".to_string()
    });
    request.island_trigger_reason = Some(input.reason.as_str().to_string());
    request.island_source = input.source.clone();
    request
}

fn rejected_gateway_adoption(
    challenger: &crate::continuation::ContinueDecisionResult,
    challenger_state: &IslandContinueState,
    status: &CaptureStatus,
    input: &IslandContinueStateInput,
) -> Option<(IslandContinueState, Vec<String>)> {
    if matches!(
        input.reason,
        IslandContinueReason::UserPressedContinue | IslandContinueReason::MainCardDecisionUpdated
    ) {
        return None;
    }
    let remembered = LAST_CONTINUE_ISLAND_STATE
        .lock()
        .ok()
        .and_then(|slot| slot.clone())?;
    let mut reasons = Vec::new();
    let (challenger_task_id, challenger_task_revision) = adoption_task_identity(challenger);
    let same_task = remembered.task_turn_id.as_deref().is_some()
        && remembered.task_turn_id.as_deref() == challenger_task_id.as_deref();
    let challenger_evidence_ms = decision_evidence_updated_at_ms(challenger)
        .or_else(|| newest_status_evidence_ms(status))
        .unwrap_or_default();
    let causally_newer =
        challenger_evidence_ms > remembered.evidence_updated_at_ms.unwrap_or_default();

    if remembered.task_turn_id.is_some() && challenger_task_id.is_none() {
        reasons.push("rejected_lost_task_identity".to_string());
    } else if remembered.task_turn_id.is_some() && !same_task {
        if !adoption_task_is_supported(challenger) {
            reasons.push("rejected_new_task_not_supported".to_string());
        }
        if !causally_newer {
            reasons.push("rejected_new_task_not_causally_newer".to_string());
        }
    } else if same_task && challenger_task_revision < remembered.task_turn_revision {
        reasons.push("rejected_older_task_revision".to_string());
    }
    if !causally_newer {
        reasons.push("rejected_evidence_not_causally_newer".to_string());
    }
    if adoption_task_confidence(challenger) + f64::EPSILON < remembered.task_confidence {
        reasons.push("rejected_lower_task_identity_confidence".to_string());
    }
    if remembered.continue_openable && !challenger_state.allows_open_continue_target() {
        reasons.push("rejected_target_policy_downgrade".to_string());
    }
    for (present, next_present, field) in [
        (
            remembered.island_continue_state.activity_summary.is_some(),
            challenger_state.activity_summary.is_some(),
            "task",
        ),
        (
            remembered.island_continue_state.activity_state.is_some(),
            challenger_state.activity_state.is_some(),
            "state",
        ),
        (
            remembered.island_continue_state.next_action.is_some(),
            challenger_state.next_action.is_some(),
            "next",
        ),
        (
            remembered.island_continue_state.activity_where.is_some(),
            challenger_state.activity_where.is_some(),
            "where",
        ),
    ] {
        if present && !next_present {
            reasons.push(format!("rejected_lost_supported_{field}"));
        }
    }
    if source_quality_rank(&adoption_wording_source(challenger))
        < source_quality_rank(&remembered.wording_source)
    {
        reasons.push("rejected_wording_source_downgrade".to_string());
    }
    if target_source_quality_rank(&adoption_target_selection_source(challenger))
        < target_source_quality_rank(&remembered.target_selection_source)
    {
        reasons.push("rejected_target_selection_source_downgrade".to_string());
    }
    if remembered.request_trigger == "manual" && !reasons.is_empty() {
        reasons.push("retained_stronger_manual_result".to_string());
    }
    reasons.sort();
    reasons.dedup();
    (!reasons.is_empty()).then_some((remembered.island_continue_state, reasons))
}

fn adoption_task_identity(
    decision: &crate::continuation::ContinueDecisionResult,
) -> (Option<String>, Option<i64>) {
    if let Some(answer) = authoritative_task_truth_answer(decision) {
        return (
            answer
                .atomic_identity
                .task_thread_id
                .filter(|id| !id.trim().is_empty()),
            answer.atomic_identity.task_thread_revision,
        );
    }
    (
        decision
            .current_task_turn
            .as_ref()
            .map(|turn| turn.task_turn_id.clone()),
        decision
            .current_task_turn
            .as_ref()
            .map(|turn| turn.revision),
    )
}

fn adoption_task_is_supported(decision: &crate::continuation::ContinueDecisionResult) -> bool {
    authoritative_task_truth_answer(decision).map_or_else(
        || decision.task_resolution_status == "current_task_supported",
        |answer| answer.task_resolution_status != "unresolved",
    )
}

fn adoption_task_confidence(decision: &crate::continuation::ContinueDecisionResult) -> f64 {
    let Some(answer) = authoritative_task_truth_answer(decision) else {
        return decision.confidence_summary.task.score;
    };
    ["task_summary", "task_object"]
        .iter()
        .filter_map(|field| {
            answer
                .field_support
                .get(*field)
                .and_then(|support| support.confidence)
        })
        .reduce(f64::min)
        .unwrap_or_else(|| {
            if answer.task_resolution_status == "resolved" {
                0.85
            } else {
                0.6
            }
        })
}

fn adoption_wording_source(decision: &crate::continuation::ContinueDecisionResult) -> String {
    authoritative_task_truth_answer(decision)
        .map(|answer| answer.wording_source)
        .unwrap_or_else(|| decision.wording_source.clone())
}

fn adoption_target_selection_source(
    decision: &crate::continuation::ContinueDecisionResult,
) -> String {
    authoritative_task_truth_answer(decision)
        .map(|answer| answer.target_selection_source)
        .unwrap_or_else(|| decision.target_selection_source.clone())
}

fn source_quality_rank(source: &str) -> u8 {
    if source.contains("model") {
        2
    } else if source.contains("fallback") || source.contains("abstained") {
        0
    } else {
        1
    }
}

fn target_source_quality_rank(source: &str) -> u8 {
    if source.contains("validated") {
        2
    } else if source.contains("abstained") {
        0
    } else {
        1
    }
}

pub fn island_state_from_decision_status(
    decision: &crate::continuation::ContinueDecisionResult,
    status: &CaptureStatus,
    decision_stale: bool,
) -> IslandContinueState {
    let newest_evidence_ms = newest_status_evidence_ms(status);
    let freshness = IslandFreshness {
        evidence_watermark_ms: decision_evidence_updated_at_ms(decision).or(newest_evidence_ms),
        newest_evidence_ms,
        decision_updated_at_ms: Some(super::now_millis()),
        decision_stale,
    };
    super::island_state_from_continue_decision(decision, freshness, context_from_status(status))
}

fn remembered_fresh_state(
    status: &CaptureStatus,
    input: &IslandContinueStateInput,
    feedback_or_open_watermark_ms: Option<i64>,
) -> Option<IslandContinueState> {
    let remembered = LAST_CONTINUE_ISLAND_STATE
        .lock()
        .ok()
        .and_then(|slot| slot.clone())?;
    if input
        .existing_decision_id
        .as_deref()
        .is_some_and(|id| id != remembered.decision_id)
    {
        return None;
    }
    if remembered_state_is_stale(&remembered, status, feedback_or_open_watermark_ms) {
        return None;
    }

    let mut state = remembered.island_continue_state;
    state.decision_cache_hit = true;
    annotate_gateway_state(&mut state, input);
    Some(state)
}

pub fn remembered_state_is_stale(
    remembered: &RememberedContinueIslandState,
    status: &CaptureStatus,
    feedback_or_open_watermark_ms: Option<i64>,
) -> bool {
    let status_session_id = status
        .active_session
        .as_ref()
        .or(status.latest_session.as_ref())
        .map(|session| session.id.as_str())
        .or_else(|| {
            status
                .latest_frame
                .as_ref()
                .and_then(|frame| frame.session_id.as_deref())
        });
    if remembered.session_id.as_deref() != status_session_id {
        return true;
    }
    let latest_capture_at = status
        .latest_frame
        .as_ref()
        .map(|frame| frame.captured_at)
        .unwrap_or_default();
    let remembered_evidence_at = remembered.evidence_updated_at_ms.unwrap_or_default();
    status.frame_count.max(0) as u64 > remembered.frame_count
        || status.signal_count.max(0) as u64 > remembered.signal_count
        || status.event_count.max(0) as u64 > remembered.event_count
        || latest_capture_at > remembered_evidence_at
        || match (
            feedback_or_open_watermark_ms,
            remembered.feedback_or_open_watermark_ms,
        ) {
            (Some(latest), Some(remembered)) => latest > remembered,
            (Some(_), None) => true,
            _ => false,
        }
}

fn remember_gateway_decision(
    decision: &crate::continuation::ContinueDecisionResult,
    state: &IslandContinueState,
    status: &CaptureStatus,
    feedback_or_open_watermark_ms: Option<i64>,
) {
    super::remember_continue_decision_id(&decision.decision_id);
    let (task_turn_id, task_turn_revision) = adoption_task_identity(decision);
    if let Ok(mut slot) = LAST_CONTINUE_ISLAND_STATE.lock() {
        *slot = Some(RememberedContinueIslandState {
            session_id: status
                .active_session
                .as_ref()
                .or(status.latest_session.as_ref())
                .map(|session| session.id.clone()),
            decision_id: decision.decision_id.clone(),
            request_trigger: decision.request_trigger.clone(),
            task_turn_id,
            task_turn_revision,
            task_confidence: adoption_task_confidence(decision),
            wording_source: adoption_wording_source(decision),
            target_selection_source: adoption_target_selection_source(decision),
            resume_headline: Some(headline_from_island_state(state).to_string()),
            resume_detail: state.next_action.clone(),
            resume_point: state
                .resume_work_target
                .as_ref()
                .or(state.return_target.as_ref())
                .map(|target| target.title.clone()),
            resume_warning: state
                .missing_evidence
                .first()
                .or_else(|| state.warnings.first())
                .cloned(),
            continue_freshness: continue_freshness_from_island_state(state),
            evidence_updated_at_ms: state
                .evidence_watermark_ms
                .or_else(|| newest_status_evidence_ms(status)),
            decision_updated_at_ms: Some(state.generated_at_ms),
            continue_openable: state.allows_open_continue_target(),
            feedback_or_open_watermark_ms,
            frame_count: status.frame_count.max(0) as u64,
            signal_count: status.signal_count.max(0) as u64,
            event_count: status.event_count.max(0) as u64,
            island_continue_state: state.clone(),
        });
    }
}

fn context_from_status(status: &CaptureStatus) -> IslandStateContext {
    IslandStateContext {
        local_memory_running: status.running,
        has_local_memory: status.frame_count > 0
            || status.event_count > 0
            || status.signal_count > 0,
    }
}

fn newest_status_evidence_ms(status: &CaptureStatus) -> Option<i64> {
    status.latest_frame.as_ref().map(|frame| frame.captured_at)
}

fn annotate_gateway_state(state: &mut IslandContinueState, input: &IslandContinueStateInput) {
    state.provenance_label = Some(format!(
        "island:{}:{}",
        input.source.as_deref().unwrap_or("unknown"),
        input.reason.as_str()
    ));
    if input.force_refresh && state.display_state != IslandDisplayState::NeedsRefresh {
        state
            .warnings
            .push("island_gateway_force_refresh".to_string());
        state.warnings.sort();
        state.warnings.dedup();
    }
    for action in &mut state.available_actions {
        if matches!(action.kind, super::IslandActionKind::OpenContinueTarget) {
            action.decision_id = state.decision_id.clone();
        }
    }
}

fn emit_gateway_state(app: &AppHandle, state: &IslandContinueState) {
    let _ = app.emit("island-continue-state", state.clone());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(
        frame_count: i64,
        signal_count: i64,
        event_count: i64,
        captured_at: Option<i64>,
    ) -> CaptureStatus {
        CaptureStatus {
            running: frame_count > 0 || event_count > 0,
            frame_count,
            recent_app_labels: Vec::new(),
            signal_count,
            event_count,
            transition_count: 0,
            content_unit_count: 0,
            session_count: 0,
            active_session: None,
            latest_session: None,
            last_export: None,
            started_at: None,
            last_error: None,
            latest_frame: captured_at.map(|captured_at| crate::capture::CaptureFrame {
                id: 1,
                captured_at,
                snapshot_path: String::new(),
                app_name: Some("Codex".to_string()),
                window_name: Some("Smalltalk".to_string()),
                browser_url: None,
                document_path: None,
                focused: true,
                capture_trigger: "test".to_string(),
                text_source: None,
                accessibility_text: None,
                accessibility_tree_json: None,
                full_text: None,
                content_hash: None,
                image_hash: None,
                capture_provider: None,
                active_window_capture_provider: None,
                scope: None,
                display_id: None,
                window_id: None,
                app_pid: None,
                app_bundle_id: None,
                screen_scale: None,
                pixel_width: None,
                pixel_height: None,
                full_screenshot_path: None,
                active_window_crop_path: None,
                active_element_crop_path: None,
                phash: None,
                privacy_status: Some("normal".to_string()),
                capture_trigger_id: None,
                previous_frame_id: None,
                session_id: Some("session-test".to_string()),
                sck_display_id: None,
                sck_window_id: None,
                sck_owning_bundle_id: None,
                sck_filter_summary_json: None,
                sck_configuration_summary_json: None,
                sck_frame_metadata_json: None,
                sck_capture_mode: None,
                sck_audio_policy: None,
            }),
            skipped_samples: 0,
            last_skipped_at: None,
            data_dir: String::new(),
            database_path: String::new(),
            screenshot_tool: false,
            accessibility_tool: false,
            ocr_tool: false,
            runtime_diagnostics: crate::capture::RuntimeDiagnostics::default(),
        }
    }

    fn remembered() -> RememberedContinueIslandState {
        let mut island_continue_state = IslandContinueState::refresh_needed(
            IslandFreshness {
                evidence_watermark_ms: Some(1_000),
                newest_evidence_ms: Some(1_000),
                decision_updated_at_ms: Some(1_100),
                decision_stale: false,
            },
            IslandStateContext {
                local_memory_running: true,
                has_local_memory: true,
            },
            Some("decision-test".to_string()),
        );
        island_continue_state.display_state = IslandDisplayState::ContinueReady;
        island_continue_state.decision_stale = false;
        island_continue_state.available_actions =
            vec![super::super::IslandAvailableAction::enabled(
                super::super::IslandActionKind::OpenContinueTarget,
                "Open Continue target",
                Some("decision-test".to_string()),
            )];
        RememberedContinueIslandState {
            session_id: Some("session-test".to_string()),
            decision_id: "decision-test".to_string(),
            request_trigger: "manual".to_string(),
            task_turn_id: Some("task-test".to_string()),
            task_turn_revision: Some(2),
            task_confidence: 0.9,
            wording_source: "model_assisted".to_string(),
            target_selection_source: "local_validated_target_policy".to_string(),
            resume_headline: Some("Ready to continue".to_string()),
            resume_detail: Some("Continue from the gateway.".to_string()),
            resume_point: Some("session_island.rs".to_string()),
            resume_warning: None,
            continue_freshness: "current".to_string(),
            evidence_updated_at_ms: Some(1_000),
            decision_updated_at_ms: Some(1_100),
            continue_openable: true,
            feedback_or_open_watermark_ms: Some(900),
            frame_count: 1,
            signal_count: 2,
            event_count: 3,
            island_continue_state,
        }
    }

    #[test]
    fn island_continue_gateway_reuses_existing_fresh_decision_state() {
        let remembered = remembered();
        assert!(!remembered_state_is_stale(
            &remembered,
            &status(1, 2, 3, Some(1_000)),
            Some(900)
        ));
    }

    #[test]
    fn island_continue_gateway_rejects_remembered_state_from_adjacent_session() {
        let mut remembered = remembered();
        remembered.session_id = Some("session-a".to_string());
        assert!(remembered_state_is_stale(
            &remembered,
            &status(1, 2, 3, Some(1_000)),
            Some(900)
        ));
    }

    #[test]
    fn island_continue_gateway_refreshes_for_event_only_evidence_growth() {
        let remembered = remembered();
        assert!(remembered_state_is_stale(
            &remembered,
            &status(1, 2, 4, Some(1_000)),
            Some(900)
        ));
    }

    #[test]
    fn island_continue_gateway_refreshes_for_new_frame_evidence() {
        let remembered = remembered();
        assert!(remembered_state_is_stale(
            &remembered,
            &status(2, 2, 3, Some(1_200)),
            Some(900)
        ));
    }

    #[test]
    fn island_continue_gateway_refreshes_for_feedback_or_open_watermark_growth() {
        let remembered = remembered();
        assert!(remembered_state_is_stale(
            &remembered,
            &status(1, 2, 3, Some(1_000)),
            Some(901)
        ));
    }

    #[test]
    fn island_continue_gateway_no_evidence_state_has_no_open_action() {
        let mut state = IslandContinueState::no_evidence(
            IslandFreshness {
                evidence_watermark_ms: None,
                newest_evidence_ms: None,
                decision_updated_at_ms: Some(2_000),
                decision_stale: false,
            },
            IslandStateContext::default(),
        );
        annotate_gateway_state(
            &mut state,
            &IslandContinueStateInput {
                reason: IslandContinueReason::InitialRender,
                existing_decision_id: None,
                allow_refresh: false,
                force_refresh: false,
                source: Some("test".to_string()),
            },
        );

        assert_eq!(state.display_state, IslandDisplayState::NoLocalMemory);
        assert!(!state.allows_open_continue_target());
        assert_eq!(
            state.provenance_label.as_deref(),
            Some("island:test:initial_render")
        );
    }

    #[test]
    fn explicit_island_continue_uses_manual_semantic_trigger_with_island_provenance() {
        let input = IslandContinueStateInput::for_user_continue(Some("decision-prior".into()));

        let request = continue_decision_request_for_input(&input);

        assert_eq!(request.request_trigger.as_deref(), Some("manual"));
        assert_eq!(
            request.island_trigger_reason.as_deref(),
            Some("user_pressed_continue")
        );
        assert_eq!(request.island_source.as_deref(), Some("native_island"));
        assert_eq!(request.audit_output_enabled, Some(false));
        assert!(matches!(
            request.audit_mode,
            Some(crate::continuation::ContinueAuditMode::MftiReview)
        ));
    }
}
