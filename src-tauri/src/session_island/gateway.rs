use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::capture::{CaptureState, CaptureStatus};

use super::{
    continue_freshness_from_island_state, decision_evidence_updated_at_ms,
    headline_from_island_state, island_continue_decision_request, IslandContinueState,
    IslandDisplayState, IslandFreshness, IslandStateContext, RememberedContinueIslandState,
    LAST_CONTINUE_ISLAND_STATE,
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

    let mut request = island_continue_decision_request();
    request.audit_output_enabled = Some(true);
    request.island_trigger_reason = Some(input.reason.as_str().to_string());
    request.island_source = input.source.clone();
    let decision = crate::capture::get_continue_decision(app.clone(), Some(request))?;
    let mut state = island_state_from_decision_status(&decision, &status, false);
    annotate_gateway_state(&mut state, &input);
    state.audit_path = decision.continue_output_path.clone();
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
    if let Ok(mut slot) = LAST_CONTINUE_ISLAND_STATE.lock() {
        *slot = Some(RememberedContinueIslandState {
            decision_id: decision.decision_id.clone(),
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
            decision_id: "decision-test".to_string(),
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
}
