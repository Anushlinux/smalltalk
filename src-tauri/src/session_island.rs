use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

use crate::capture::{CaptureStatus, CloudResumeResult, OpenResumePointInput};

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static EXPANDED: AtomicBool = AtomicBool::new(false);
#[allow(dead_code)]
static LAST_CLOUD_RESUME_OUTPUT_PATH: Mutex<Option<String>> = Mutex::new(None);
static LAST_CONTINUE_DECISION_ID: Mutex<Option<String>> = Mutex::new(None);
static LAST_CONTINUE_TARGET_ARTIFACT_ID: Mutex<Option<String>> = Mutex::new(None);
static LAST_CONTINUE_ISLAND_STATE: Mutex<Option<RememberedContinueIslandState>> = Mutex::new(None);

#[derive(Debug, Clone)]
struct RememberedContinueIslandState {
    decision_id: String,
    resume_headline: Option<String>,
    resume_detail: Option<String>,
    resume_point: Option<String>,
    resume_warning: Option<String>,
    continue_freshness: String,
    evidence_updated_at_ms: Option<i64>,
    decision_updated_at_ms: Option<i64>,
    continue_openable: bool,
    frame_count: u64,
    signal_count: u64,
    event_count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionIslandSnapshot {
    pub state: SessionIslandState,
    pub session_id: Option<String>,
    pub elapsed_ms: u64,
    pub frame_count: u64,
    pub event_count: u64,
    pub trail_app_count: u64,
    pub trail_moment_count: u64,
    pub trail_labels: Vec<String>,
    pub last_frame_id: Option<i64>,
    pub current_app: Option<String>,
    pub current_window: Option<String>,
    pub current_surface_kind: Option<String>,
    pub last_trigger: Option<String>,
    pub last_capture_at_ms: Option<i64>,
    pub capture_pulse_nonce: Option<u64>,
    pub last_error: Option<String>,
    pub resume_headline: Option<String>,
    pub resume_detail: Option<String>,
    pub resume_point: Option<String>,
    pub resume_source: Option<String>,
    pub resume_model: Option<String>,
    pub resume_response_id: Option<String>,
    pub continue_decision_id: Option<String>,
    pub continue_freshness: Option<String>,
    pub evidence_updated_at_ms: Option<i64>,
    pub decision_updated_at_ms: Option<i64>,
    pub continue_openable: Option<bool>,
    pub resume_warning: Option<String>,
    pub privacy_label: Option<String>,
    pub is_sensitive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum SessionIslandState {
    Hidden,
    Ready,
    Starting,
    RecordingCompact,
    RecordingExpanded,
    Processing,
    StoppedToast,
    TrailReconstructing,
    ResumeReady,
    Error,
}

#[derive(Debug, Deserialize)]
struct SessionIslandAction {
    action: SessionIslandActionKind,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SessionIslandActionKind {
    Continue,
    StartCapture,
    StopCapture,
    CaptureOnce,
    ReconstructTrail,
    ShowTrail,
    OpenResumePoint,
    OpenMainWindow,
    ResumeMe,
    ToggleExpanded,
    Collapse,
}

impl SessionIslandSnapshot {
    pub fn hidden() -> Self {
        Self {
            state: SessionIslandState::Hidden,
            session_id: None,
            elapsed_ms: 0,
            frame_count: 0,
            event_count: 0,
            trail_app_count: 0,
            trail_moment_count: 0,
            trail_labels: Vec::new(),
            last_frame_id: None,
            current_app: None,
            current_window: None,
            current_surface_kind: None,
            last_trigger: None,
            last_capture_at_ms: None,
            capture_pulse_nonce: None,
            last_error: None,
            resume_headline: None,
            resume_detail: None,
            resume_point: None,
            resume_source: None,
            resume_model: None,
            resume_response_id: None,
            continue_decision_id: None,
            continue_freshness: None,
            evidence_updated_at_ms: None,
            decision_updated_at_ms: None,
            continue_openable: None,
            resume_warning: None,
            privacy_label: None,
            is_sensitive: false,
        }
    }

    pub fn ready() -> Self {
        Self {
            state: SessionIslandState::Ready,
            ..Self::hidden()
        }
    }

    pub fn starting() -> Self {
        Self {
            state: SessionIslandState::Starting,
            ..Self::hidden()
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            state: SessionIslandState::Error,
            last_error: Some(message),
            ..Self::hidden()
        }
    }
}

pub fn init_session_island(app: AppHandle) {
    let _ = APP_HANDLE.set(app.clone());

    #[cfg(target_os = "macos")]
    unsafe {
        smalltalk_island_init();
        smalltalk_island_set_action_callback(handle_native_action);
    }

    match crate::capture::capture_status(app.clone(), app.state::<crate::capture::CaptureState>()) {
        Ok(status) => {
            let state = if status.running {
                SessionIslandState::RecordingCompact
            } else {
                SessionIslandState::Ready
            };
            update_session_island_from_status(&status, state);
            show_session_island();
        }
        Err(error) => {
            eprintln!("[session_island] initial status unavailable: {}", error);
            update_session_island(SessionIslandSnapshot::ready());
            show_session_island();
        }
    }
}

pub fn update_session_island(snapshot: SessionIslandSnapshot) {
    if matches!(
        snapshot.state,
        SessionIslandState::Hidden
            | SessionIslandState::Starting
            | SessionIslandState::Processing
            | SessionIslandState::TrailReconstructing
            | SessionIslandState::StoppedToast
    ) {
        EXPANDED.store(false, Ordering::Relaxed);
    }

    let Ok(json) = serde_json::to_string(&snapshot) else {
        eprintln!("[session_island] failed to serialize snapshot");
        return;
    };
    let Ok(json) = CString::new(json) else {
        eprintln!("[session_island] snapshot contained an unexpected nul byte");
        return;
    };

    #[cfg(target_os = "macos")]
    unsafe {
        smalltalk_island_update_json(json.as_ptr());
    }

    #[cfg(not(target_os = "macos"))]
    let _ = json;
}

pub fn update_session_island_from_status(status: &CaptureStatus, state: SessionIslandState) {
    update_session_island(snapshot_from_status(status, state));
}

pub fn return_to_ready_after_stop(app: AppHandle) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(1700));
        let state = app.state::<crate::capture::CaptureState>();
        match crate::capture::capture_status(app.clone(), state) {
            Ok(status) => {
                let island_state = if status.running {
                    SessionIslandState::RecordingCompact
                } else {
                    SessionIslandState::Ready
                };
                update_session_island_from_status(&status, island_state);
                show_session_island();
            }
            Err(error) => update_session_island(SessionIslandSnapshot::error(error)),
        }
    });
}

pub fn show_session_island() {
    #[cfg(target_os = "macos")]
    unsafe {
        smalltalk_island_show();
    }
}

#[allow(dead_code)]
pub fn hide_session_island() {
    #[cfg(target_os = "macos")]
    unsafe {
        smalltalk_island_hide();
    }
}

pub fn set_session_island_expanded(expanded: bool) {
    EXPANDED.store(expanded, Ordering::Relaxed);

    #[cfg(target_os = "macos")]
    unsafe {
        smalltalk_island_set_expanded(expanded);
    }

    #[cfg(not(target_os = "macos"))]
    let _ = expanded;
}

#[allow(dead_code)]
pub fn reposition_session_island() {
    #[cfg(target_os = "macos")]
    unsafe {
        smalltalk_island_reposition();
    }
}

#[allow(dead_code)]
pub fn shutdown_session_island() {
    #[cfg(target_os = "macos")]
    unsafe {
        smalltalk_island_shutdown();
    }
}

fn snapshot_from_status(
    status: &CaptureStatus,
    state: SessionIslandState,
) -> SessionIslandSnapshot {
    let frame = status.latest_frame.as_ref();
    let privacy_label = frame
        .and_then(|frame| frame.privacy_status.clone())
        .filter(|label| !label.trim().is_empty());
    let is_sensitive = privacy_label
        .as_deref()
        .map(is_sensitive_privacy_label)
        .unwrap_or(false);
    let session_id = status
        .active_session
        .as_ref()
        .or(status.latest_session.as_ref())
        .map(|session| session.id.clone());
    let elapsed_ms = status
        .started_at
        .map(|started_at| now_millis().saturating_sub(started_at) as u64)
        .or_else(|| {
            status.latest_session.as_ref().and_then(|session| {
                session
                    .stopped_at
                    .map(|stopped_at| stopped_at.saturating_sub(session.started_at) as u64)
            })
        })
        .unwrap_or(0);

    let mut snapshot = SessionIslandSnapshot {
        state,
        session_id,
        elapsed_ms,
        frame_count: status.frame_count.max(0) as u64,
        event_count: status.event_count.max(0) as u64,
        trail_app_count: status.recent_app_labels.len() as u64,
        trail_moment_count: status.signal_count.max(0) as u64,
        trail_labels: status.recent_app_labels.clone(),
        last_frame_id: frame.map(|frame| frame.id),
        current_app: if is_sensitive {
            None
        } else {
            frame.and_then(|frame| clean_one_line(frame.app_name.as_deref()))
        },
        current_window: if is_sensitive {
            None
        } else {
            frame.and_then(|frame| clean_one_line(frame.window_name.as_deref()))
        },
        current_surface_kind: frame.and_then(|frame| {
            frame
                .scope
                .as_deref()
                .or(frame.text_source.as_deref())
                .and_then(|value| clean_one_line(Some(value)))
        }),
        last_trigger: frame.and_then(|frame| clean_one_line(Some(&frame.capture_trigger))),
        last_capture_at_ms: frame.map(|frame| frame.captured_at),
        capture_pulse_nonce: frame
            .map(|frame| frame.id.max(0) as u64)
            .filter(|nonce| *nonce > 0),
        last_error: status.last_error.clone(),
        resume_headline: None,
        resume_detail: None,
        resume_point: None,
        resume_source: None,
        resume_model: None,
        resume_response_id: None,
        continue_decision_id: None,
        continue_freshness: None,
        evidence_updated_at_ms: frame.map(|frame| frame.captured_at),
        decision_updated_at_ms: None,
        continue_openable: None,
        resume_warning: None,
        privacy_label,
        is_sensitive,
    };
    apply_remembered_continue_to_status_snapshot(&mut snapshot, status);
    snapshot
}

fn clean_one_line(value: Option<&str>) -> Option<String> {
    value
        .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|value| !value.is_empty())
}

fn is_sensitive_privacy_label(label: &str) -> bool {
    !matches!(label, "normal" | "ok" | "allowed")
}

fn apply_remembered_continue_to_status_snapshot(
    snapshot: &mut SessionIslandSnapshot,
    _status: &CaptureStatus,
) {
    if matches!(
        snapshot.state,
        SessionIslandState::Hidden
            | SessionIslandState::Starting
            | SessionIslandState::Processing
            | SessionIslandState::TrailReconstructing
            | SessionIslandState::StoppedToast
            | SessionIslandState::Error
    ) {
        return;
    }

    let remembered = LAST_CONTINUE_ISLAND_STATE
        .lock()
        .ok()
        .and_then(|slot| slot.clone());
    let Some(remembered) = remembered else {
        return;
    };

    let latest_capture_at = snapshot.last_capture_at_ms.unwrap_or_default();
    let remembered_evidence_at = remembered.evidence_updated_at_ms.unwrap_or_default();
    let has_new_evidence = snapshot.frame_count > remembered.frame_count
        || snapshot.trail_moment_count > remembered.signal_count
        || snapshot.event_count > remembered.event_count
        || latest_capture_at > remembered_evidence_at;

    snapshot.continue_decision_id = Some(remembered.decision_id);
    snapshot.continue_openable = Some(remembered.continue_openable);
    snapshot.decision_updated_at_ms = remembered.decision_updated_at_ms;
    snapshot.evidence_updated_at_ms = Some(latest_capture_at.max(remembered_evidence_at));
    snapshot.resume_point = remembered.resume_point.clone();

    if has_new_evidence {
        snapshot.continue_freshness = Some("new_evidence".to_string());
        snapshot.resume_headline = Some("New evidence".to_string());
        snapshot.resume_detail = Some("Refresh Continue".to_string());
        snapshot.resume_warning = None;
    } else {
        snapshot.continue_freshness = Some(remembered.continue_freshness);
        snapshot.resume_headline = remembered.resume_headline;
        snapshot.resume_detail = remembered.resume_detail;
        snapshot.resume_warning = remembered.resume_warning;
    }
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

extern "C" fn handle_native_action(action_json: *const c_char) {
    if action_json.is_null() {
        return;
    }

    let payload = unsafe { CStr::from_ptr(action_json) }
        .to_string_lossy()
        .into_owned();
    let action = match serde_json::from_str::<SessionIslandAction>(&payload) {
        Ok(action) => action,
        Err(error) => {
            eprintln!("[session_island] ignored malformed action: {}", error);
            return;
        }
    };

    match action.action {
        SessionIslandActionKind::StartCapture => start_capture_from_island(),
        SessionIslandActionKind::StopCapture => stop_capture_from_island(),
        SessionIslandActionKind::CaptureOnce => capture_once_from_island(),
        SessionIslandActionKind::Continue => continue_from_island(),
        SessionIslandActionKind::ReconstructTrail => continue_from_island(),
        SessionIslandActionKind::ShowTrail => open_main_window(),
        SessionIslandActionKind::OpenResumePoint => open_resume_point_from_island(),
        SessionIslandActionKind::OpenMainWindow => open_main_window(),
        SessionIslandActionKind::ResumeMe => open_resume_point_from_island(),
        SessionIslandActionKind::ToggleExpanded => toggle_expanded_from_native(),
        SessionIslandActionKind::Collapse => set_session_island_expanded(false),
    }
}

fn continue_from_island() {
    let Some(app) = APP_HANDLE.get().cloned() else {
        eprintln!("[session_island] continue requested before AppHandle was ready");
        return;
    };

    let state = app.state::<crate::capture::CaptureState>();
    match crate::capture::capture_status(app.clone(), state) {
        Ok(status) => {
            update_session_island_from_status(&status, SessionIslandState::TrailReconstructing);
            show_session_island();
        }
        Err(error) => update_session_island(SessionIslandSnapshot::error(error)),
    }

    thread::spawn(move || {
        match crate::capture::get_continue_decision(
            app.clone(),
            Some(crate::continuation::ContinueDecisionRequest {
                mode: Some("normal".to_string()),
                rebuild_layers: Some(false),
                micro_inference_enabled: Some(true),
                max_candidates_for_model: Some(5),
                audit_output_enabled: Some(true),
                ..Default::default()
            }),
        ) {
            Ok(decision) => {
                remember_continue_decision(&decision);
                let state = app.state::<crate::capture::CaptureState>();
                let next_status = crate::capture::capture_status(app.clone(), state)
                    .unwrap_or_else(|_| crate::capture::CaptureStatus {
                        running: false,
                        frame_count: 0,
                        recent_app_labels: Vec::new(),
                        signal_count: 0,
                        event_count: 0,
                        transition_count: 0,
                        content_unit_count: 0,
                        session_count: 0,
                        active_session: None,
                        latest_session: None,
                        last_export: None,
                        started_at: None,
                        last_error: None,
                        latest_frame: None,
                        skipped_samples: 0,
                        last_skipped_at: None,
                        data_dir: String::new(),
                        database_path: String::new(),
                        screenshot_tool: false,
                        accessibility_tool: false,
                        ocr_tool: false,
                        runtime_diagnostics: crate::capture::RuntimeDiagnostics::default(),
                    });
                let mut snapshot =
                    snapshot_from_status(&next_status, SessionIslandState::ResumeReady);
                apply_continue_decision_to_snapshot(&mut snapshot, &decision);
                let _ = app.emit("session-island-continue-ready", decision.clone());
                let _ = app.emit("smalltalk-continue-updated", decision.clone());
                update_session_island(snapshot);
                show_session_island();
            }
            Err(error) => {
                eprintln!("[session_island] continue failed: {}", error);
                update_session_island(SessionIslandSnapshot::error(error));
            }
        }
    });
}

fn apply_continue_decision_to_snapshot(
    snapshot: &mut SessionIslandSnapshot,
    decision: &crate::continuation::ContinueDecisionResult,
) {
    let target = decision
        .resume_work_target
        .as_ref()
        .or(decision.return_target.as_ref());
    let continue_openable = target.is_some_and(|target| {
        target.openability == "openable"
            && (target.browser_url.is_some() || target.document_path.is_some())
    });
    let thin = decision.confidence < 0.55
        || decision.confidence_label.eq_ignore_ascii_case("thin")
        || decision.validation_status.contains("thin")
        || !decision.missing_evidence.is_empty()
        || !decision.validation_failures.is_empty();
    let continue_freshness = if thin {
        "thin_evidence"
    } else if continue_openable {
        "current"
    } else {
        "needs_evidence"
    }
    .to_string();

    snapshot.continue_decision_id = Some(decision.decision_id.clone());
    snapshot.continue_freshness = Some(continue_freshness.clone());
    snapshot.decision_updated_at_ms = Some(now_millis());
    snapshot.evidence_updated_at_ms = decision_evidence_updated_at_ms(decision)
        .or(snapshot.last_capture_at_ms)
        .or(snapshot.decision_updated_at_ms);
    snapshot.continue_openable = Some(continue_openable);
    snapshot.resume_source = Some("continue".to_string());
    snapshot.resume_model = decision.model.clone();
    snapshot.resume_response_id = decision.response_id.clone();
    snapshot.resume_headline = Some(match decision.confidence_label.as_str() {
        "high" => "Ready to continue".to_string(),
        "medium" => "Likely continuation found".to_string(),
        _ => "Evidence is thin".to_string(),
    });
    snapshot.resume_detail = clean_one_line(
        Some(decision.handoff.last_state_line.as_str())
            .filter(|line| !line.trim().is_empty())
            .or_else(|| {
                decision
                    .selected_workstream
                    .as_ref()
                    .and_then(|workstream| workstream.title_candidate.as_deref())
            })
            .or(decision.next_action.as_deref()),
    );
    snapshot.resume_point = clean_one_line(
        target
            .and_then(|target| {
                target
                    .title
                    .as_deref()
                    .or(target.browser_url.as_deref())
                    .or(target.document_path.as_deref())
                    .or(target.artifact_kind.as_deref())
            }),
    );
    snapshot.resume_warning = decision
        .missing_evidence
        .first()
        .or_else(|| decision.warnings.first())
        .and_then(|warning| clean_one_line(Some(warning)));
    remember_continue_decision_from_snapshot(decision, snapshot, continue_freshness, continue_openable);
}

fn decision_evidence_updated_at_ms(
    decision: &crate::continuation::ContinueDecisionResult,
) -> Option<i64> {
    decision
        .evidence_freshness_ledger
        .as_ref()
        .and_then(|ledger| {
            ledger
                .get("latest_any_evidence_ms")
                .or_else(|| ledger.get("decision_watermark_ms"))
                .and_then(|value| value.as_i64())
        })
}

#[allow(dead_code)]
fn apply_cloud_resume_to_snapshot(
    snapshot: &mut SessionIslandSnapshot,
    result: &CloudResumeResult,
) {
    snapshot.resume_source = Some(result.source.clone());
    snapshot.resume_model = result.model.clone();
    snapshot.resume_response_id = result.response_id.clone();
    snapshot.resume_warning = result
        .warnings
        .iter()
        .find_map(|warning| clean_one_line(Some(warning)));

    if result.source == "cloud" {
        let current_label = cloud_target_label(&result.current_focus);
        let return_label = cloud_target_label(&result.resume_target_if_returning);
        let split_targets = current_label.is_some()
            && return_label.is_some()
            && current_label != return_label
            && result.decision == "ambiguous_current_focus_vs_prior_task";
        snapshot.resume_headline = if split_targets {
            Some("Current focus differs from return target".to_string())
        } else {
            cloud_answer_text(result, "focus_now")
                .or_else(|| clean_one_line(Some(&result.local_card.focus_now)))
        };
        snapshot.resume_detail = if split_targets {
            Some(format!(
                "Current focus: {}. Return target: {}.",
                current_label
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                return_label
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string())
            ))
        } else {
            cloud_answer_text(result, "what_was_i_doing")
                .or_else(|| clean_one_line(Some(&result.local_card.what_was_i_doing)))
        };
        snapshot.resume_point =
            cloud_resume_point_label(result).or_else(|| resume_point_label(&result.local_card));
    } else {
        let warning = snapshot
            .resume_warning
            .as_deref()
            .unwrap_or_default()
            .to_lowercase();
        snapshot.resume_headline = Some(
            if warning.contains("openai_api_key") || warning.contains("key") {
                "OpenAI key missing"
            } else {
                "OpenAI unavailable"
            }
            .to_string(),
        );
        snapshot.resume_detail = snapshot
            .resume_warning
            .clone()
            .or_else(|| clean_one_line(Some(&result.local_card.what_was_i_doing)));
        snapshot.resume_point = resume_point_label(&result.local_card);
    }
}

#[allow(dead_code)]
fn cloud_answer_text(result: &CloudResumeResult, key: &str) -> Option<String> {
    result
        .answer
        .get(key)
        .and_then(|value| value.as_str())
        .and_then(|value| clean_one_line(Some(value)))
}

#[allow(dead_code)]
fn cloud_resume_point_label(result: &CloudResumeResult) -> Option<String> {
    cloud_target_label(&result.resume_target_if_returning)
        .or_else(|| cloud_target_label(&result.resume_target))
}

#[allow(dead_code)]
fn cloud_target_label(target: &serde_json::Value) -> Option<String> {
    target
        .get("line_anchor")
        .and_then(|value| value.get("quote"))
        .and_then(|value| value.as_str())
        .or_else(|| {
            target
                .get("exact_visible_words")
                .and_then(|value| value.as_str())
        })
        .or_else(|| target.get("exact_words").and_then(|value| value.as_str()))
        .or_else(|| target.get("title").and_then(|value| value.as_str()))
        .or_else(|| target.get("app").and_then(|value| value.as_str()))
        .and_then(|value| clean_one_line(Some(value)))
}

#[allow(dead_code)]
fn resume_point_label(card: &crate::capture::NativeResumeCard) -> Option<String> {
    clean_one_line(
        card.continue_from
            .line_anchor
            .as_ref()
            .and_then(|anchor| anchor.quote.as_deref())
            .or(card.continue_from.quote.as_deref())
            .or(card.continue_from.title.as_deref())
            .or(card.continue_from.window_name.as_deref())
            .or(card.continue_from.app_name.as_deref())
            .or(card.continue_from.url.as_deref())
            .or(card.continue_from.document_path.as_deref()),
    )
}

fn open_resume_point_from_island() {
    let Some(app) = APP_HANDLE.get().cloned() else {
        eprintln!("[session_island] open resume point requested before AppHandle was ready");
        return;
    };
    thread::spawn(move || {
        let (continue_decision_id, target_artifact_id) = match remembered_continue_decision_id() {
            Some(id) => (Some(id), remembered_continue_target_artifact_id()),
            None => match crate::capture::get_continue_decision(
                app.clone(),
                Some(crate::continuation::ContinueDecisionRequest {
                    mode: Some("normal".to_string()),
                    rebuild_layers: Some(false),
                    micro_inference_enabled: Some(true),
                    max_candidates_for_model: Some(5),
                    ..Default::default()
                }),
            ) {
                Ok(decision) => {
                    let target_artifact_id = continue_decision_target_artifact_id(&decision);
                    remember_continue_decision(&decision);
                    (Some(decision.decision_id), target_artifact_id)
                }
                Err(error) => {
                    eprintln!("[session_island] get_continue_decision failed: {}", error);
                    update_session_island(SessionIslandSnapshot::error(
                        "Continue is not ready yet. Open Smalltalk to inspect local evidence."
                            .to_string(),
                    ));
                    open_main_window();
                    return;
                }
            },
        };
        if continue_decision_id.is_none() {
            eprintln!("[session_island] strict Continue open skipped because no decision id was available");
            open_main_window();
            return;
        }
        match crate::capture::open_resume_point(
            app.clone(),
            Some(OpenResumePointInput {
                output_path: None,
                session_id: None,
                continue_decision_id,
                target_artifact_id,
                strict_continue_target: true,
                current_frame_id: None,
                target_frame_id: None,
            }),
        ) {
            Ok(result) => {
                if !result.warnings.is_empty() {
                    eprintln!(
                        "[session_island] open_resume_point warnings: {}",
                        result.warnings.join(" | ")
                    );
                }
                if result.strategy.starts_with("smalltalk_") {
                    open_main_window();
                }
            }
            Err(error) => {
                eprintln!("[session_island] open_resume_point failed: {}", error);
                open_main_window();
            }
        }
    });
}

fn remember_continue_decision(decision: &crate::continuation::ContinueDecisionResult) {
    remember_continue_decision_id(&decision.decision_id);
    if let Ok(mut slot) = LAST_CONTINUE_TARGET_ARTIFACT_ID.lock() {
        *slot = continue_decision_target_artifact_id(decision);
    }
}

fn remember_continue_decision_from_snapshot(
    decision: &crate::continuation::ContinueDecisionResult,
    snapshot: &SessionIslandSnapshot,
    continue_freshness: String,
    continue_openable: bool,
) {
    remember_continue_decision(decision);
    if let Ok(mut slot) = LAST_CONTINUE_ISLAND_STATE.lock() {
        *slot = Some(RememberedContinueIslandState {
            decision_id: decision.decision_id.clone(),
            resume_headline: snapshot.resume_headline.clone(),
            resume_detail: snapshot.resume_detail.clone(),
            resume_point: snapshot.resume_point.clone(),
            resume_warning: snapshot.resume_warning.clone(),
            continue_freshness,
            evidence_updated_at_ms: snapshot.evidence_updated_at_ms,
            decision_updated_at_ms: snapshot.decision_updated_at_ms,
            continue_openable,
            frame_count: snapshot.frame_count,
            signal_count: snapshot.trail_moment_count,
            event_count: snapshot.event_count,
        });
    }
}

fn continue_decision_target_artifact_id(
    decision: &crate::continuation::ContinueDecisionResult,
) -> Option<String> {
    decision
        .resume_work_target
        .as_ref()
        .or(decision.return_target.as_ref())
        .and_then(|target| target.artifact_id.as_ref())
        .map(|artifact_id| artifact_id.trim().to_string())
        .filter(|artifact_id| !artifact_id.is_empty())
}

fn remember_continue_decision_id(decision_id: &str) {
    if let Ok(mut slot) = LAST_CONTINUE_DECISION_ID.lock() {
        *slot = Some(decision_id.to_string());
    }
}

fn remembered_continue_decision_id() -> Option<String> {
    LAST_CONTINUE_DECISION_ID
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
}

fn remembered_continue_target_artifact_id() -> Option<String> {
    LAST_CONTINUE_TARGET_ARTIFACT_ID
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
}

#[allow(dead_code)]
fn remember_cloud_resume_output_path(result: &CloudResumeResult) {
    if let Some(path) = result
        .output_path
        .as_ref()
        .filter(|path| !path.trim().is_empty())
    {
        if let Ok(mut slot) = LAST_CLOUD_RESUME_OUTPUT_PATH.lock() {
            *slot = Some(path.clone());
        }
    }
}

#[allow(dead_code)]
fn remembered_cloud_resume_output_path() -> Option<String> {
    LAST_CLOUD_RESUME_OUTPUT_PATH
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
}

fn start_capture_from_island() {
    let Some(app) = APP_HANDLE.get().cloned() else {
        eprintln!("[session_island] start requested before AppHandle was ready");
        return;
    };

    update_session_island(SessionIslandSnapshot::starting());
    show_session_island();

    thread::spawn(move || {
        let state = app.state::<crate::capture::CaptureState>();
        match crate::capture::start_capture(app.clone(), state) {
            Ok(status) => {
                let _ = app.emit("capture-status", status.clone());
                update_session_island_from_status(&status, SessionIslandState::RecordingCompact);
            }
            Err(error) => {
                eprintln!("[session_island] start_capture failed: {}", error);
                update_session_island(SessionIslandSnapshot::error(error));
            }
        }
    });
}

fn stop_capture_from_island() {
    let Some(app) = APP_HANDLE.get().cloned() else {
        eprintln!("[session_island] stop requested before AppHandle was ready");
        return;
    };

    let state = app.state::<crate::capture::CaptureState>();
    match crate::capture::capture_status(app.clone(), state) {
        Ok(status) => update_session_island_from_status(&status, SessionIslandState::Processing),
        Err(error) => update_session_island(SessionIslandSnapshot::error(error)),
    }

    thread::spawn(move || {
        let state = app.state::<crate::capture::CaptureState>();
        match crate::capture::stop_capture(app.clone(), state) {
            Ok(output) => {
                let _ = app.emit("capture-status", output.status.clone());
            }
            Err(error) => {
                eprintln!("[session_island] stop_capture failed: {}", error);
                update_session_island(SessionIslandSnapshot::error(error));
            }
        }
    });
}

fn capture_once_from_island() {
    let Some(app) = APP_HANDLE.get().cloned() else {
        eprintln!("[session_island] capture requested before AppHandle was ready");
        return;
    };

    thread::spawn(move || {
        let state = app.state::<crate::capture::CaptureState>();
        match crate::capture::capture_once(app.clone(), state) {
            Ok(_) => {
                let state = app.state::<crate::capture::CaptureState>();
                match crate::capture::capture_status(app.clone(), state) {
                    Ok(status) => {
                        let _ = app.emit("capture-status", status.clone());
                        let island_state = if status.running {
                            SessionIslandState::RecordingCompact
                        } else {
                            SessionIslandState::Ready
                        };
                        update_session_island_from_status(&status, island_state);
                    }
                    Err(error) => update_session_island(SessionIslandSnapshot::error(error)),
                }
            }
            Err(error) => {
                eprintln!("[session_island] capture_once failed: {}", error);
                update_session_island(SessionIslandSnapshot::error(error));
            }
        }
    });
}

fn open_main_window() {
    let Some(app) = APP_HANDLE.get() else {
        return;
    };
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn toggle_expanded_from_native() {
    let expanded = !EXPANDED.load(Ordering::Relaxed);
    set_session_island_expanded(expanded);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn snapshot_from_status_uses_event_backed_signal_count_for_trail_moments() {
        let _guard = TEST_LOCK.lock().unwrap();
        clear_remembered_continue_for_test();
        let status = CaptureStatus {
            running: true,
            frame_count: 1,
            recent_app_labels: vec![
                "Helium".to_string(),
                "Codex".to_string(),
                "smalltalk".to_string(),
            ],
            signal_count: 7,
            event_count: 1370,
            transition_count: 0,
            content_unit_count: 0,
            session_count: 1,
            active_session: None,
            latest_session: None,
            last_export: None,
            started_at: Some(now_millis()),
            last_error: None,
            latest_frame: None,
            skipped_samples: 0,
            last_skipped_at: None,
            data_dir: String::new(),
            database_path: String::new(),
            screenshot_tool: true,
            accessibility_tool: true,
            ocr_tool: true,
            runtime_diagnostics: crate::capture::RuntimeDiagnostics::default(),
        };

        let snapshot = snapshot_from_status(&status, SessionIslandState::RecordingCompact);

        assert_eq!(snapshot.frame_count, 1);
        assert_eq!(snapshot.event_count, 1370);
        assert_eq!(snapshot.trail_app_count, 3);
        assert_eq!(snapshot.trail_moment_count, 7);
    }

    #[test]
    fn snapshot_from_status_preserves_current_remembered_continue_without_new_evidence() {
        let _guard = TEST_LOCK.lock().unwrap();
        remember_continue_for_test(1, 7, 12, "current");
        let status = status_for_island_freshness(1, 7, 12, 10_000);

        let snapshot = snapshot_from_status(&status, SessionIslandState::RecordingCompact);

        assert_eq!(snapshot.continue_decision_id.as_deref(), Some("decision-test"));
        assert_eq!(snapshot.continue_freshness.as_deref(), Some("current"));
        assert_eq!(snapshot.resume_headline.as_deref(), Some("Ready to continue"));
        assert_eq!(snapshot.resume_point.as_deref(), Some("PRODUCT.md"));
    }

    #[test]
    fn snapshot_from_status_marks_remembered_continue_stale_on_event_only_evidence() {
        let _guard = TEST_LOCK.lock().unwrap();
        remember_continue_for_test(1, 7, 12, "current");
        let status = status_for_island_freshness(1, 7, 13, 10_000);

        let snapshot = snapshot_from_status(&status, SessionIslandState::RecordingCompact);

        assert_eq!(snapshot.continue_decision_id.as_deref(), Some("decision-test"));
        assert_eq!(snapshot.continue_freshness.as_deref(), Some("new_evidence"));
        assert_eq!(snapshot.resume_headline.as_deref(), Some("New evidence"));
        assert_eq!(snapshot.resume_detail.as_deref(), Some("Refresh Continue"));
        assert_eq!(snapshot.resume_point.as_deref(), Some("PRODUCT.md"));
    }

    fn clear_remembered_continue_for_test() {
        if let Ok(mut slot) = LAST_CONTINUE_ISLAND_STATE.lock() {
            *slot = None;
        }
    }

    fn remember_continue_for_test(
        frame_count: u64,
        signal_count: u64,
        event_count: u64,
        freshness: &str,
    ) {
        if let Ok(mut slot) = LAST_CONTINUE_ISLAND_STATE.lock() {
            *slot = Some(RememberedContinueIslandState {
                decision_id: "decision-test".to_string(),
                resume_headline: Some("Ready to continue".to_string()),
                resume_detail: Some("Editing was in progress.".to_string()),
                resume_point: Some("PRODUCT.md".to_string()),
                resume_warning: None,
                continue_freshness: freshness.to_string(),
                evidence_updated_at_ms: Some(10_000),
                decision_updated_at_ms: Some(11_000),
                continue_openable: true,
                frame_count,
                signal_count,
                event_count,
            });
        }
    }

    fn status_for_island_freshness(
        frame_count: i64,
        signal_count: i64,
        event_count: i64,
        latest_capture_at_ms: i64,
    ) -> CaptureStatus {
        CaptureStatus {
            running: true,
            frame_count,
            recent_app_labels: vec!["Codex".to_string()],
            signal_count,
            event_count,
            transition_count: 0,
            content_unit_count: 0,
            session_count: 1,
            active_session: None,
            latest_session: None,
            last_export: None,
            started_at: Some(now_millis()),
            last_error: None,
            latest_frame: Some(crate::capture::CaptureFrame {
                id: 1,
                session_id: None,
                captured_at: latest_capture_at_ms,
                snapshot_path: String::new(),
                app_name: Some("Codex".to_string()),
                window_name: Some("PRODUCT.md".to_string()),
                browser_url: None,
                document_path: Some("/Users/me/smalltalk/PRODUCT.md".to_string()),
                focused: true,
                capture_trigger: "event".to_string(),
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
                privacy_status: None,
                capture_trigger_id: None,
                previous_frame_id: None,
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
            screenshot_tool: true,
            accessibility_tool: true,
            ocr_tool: true,
            runtime_diagnostics: crate::capture::RuntimeDiagnostics::default(),
        }
    }
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
extern "C" {
    fn smalltalk_island_init();
    fn smalltalk_island_set_action_callback(cb: extern "C" fn(*const c_char));
    fn smalltalk_island_update_json(json: *const c_char);
    fn smalltalk_island_show();
    fn smalltalk_island_hide();
    fn smalltalk_island_set_expanded(expanded: bool);
    fn smalltalk_island_reposition();
    fn smalltalk_island_shutdown();
}
