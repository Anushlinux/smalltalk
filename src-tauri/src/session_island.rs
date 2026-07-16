use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

use crate::capture::{CaptureStatus, CloudResumeResult, OpenResumePointInput};

mod contract;
mod gateway;
pub use contract::{
    island_state_from_continue_decision, IslandActionKind, IslandAvailableAction,
    IslandContinueState, IslandDisplayState, IslandFreshness, IslandStateContext,
};
#[allow(unused_imports)]
pub use gateway::{IslandContinueReason, IslandContinueStateInput};

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static EXPANDED: AtomicBool = AtomicBool::new(false);
#[allow(dead_code)]
static LAST_CLOUD_RESUME_OUTPUT_PATH: Mutex<Option<String>> = Mutex::new(None);
static LAST_CONTINUE_DECISION_ID: Mutex<Option<String>> = Mutex::new(None);
static LAST_CONTINUE_ISLAND_STATE: Mutex<Option<RememberedContinueIslandState>> = Mutex::new(None);

#[derive(Debug, Clone)]
struct RememberedContinueIslandState {
    session_id: Option<String>,
    decision_id: String,
    request_trigger: String,
    task_turn_id: Option<String>,
    task_turn_revision: Option<i64>,
    task_confidence: f64,
    wording_source: String,
    target_selection_source: String,
    resume_headline: Option<String>,
    resume_detail: Option<String>,
    resume_point: Option<String>,
    resume_warning: Option<String>,
    continue_freshness: String,
    evidence_updated_at_ms: Option<i64>,
    decision_updated_at_ms: Option<i64>,
    continue_openable: bool,
    feedback_or_open_watermark_ms: Option<i64>,
    frame_count: u64,
    signal_count: u64,
    event_count: u64,
    island_continue_state: IslandContinueState,
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
    pub island_continue_state: Option<IslandContinueState>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum IslandRouteKind {
    PrimaryContinueState,
    PrimaryContinueOpen,
    PrimaryInspectEvidence,
    PrimaryLocalMemoryControl,
    SecondaryLocalMemoryControl,
    PresentationOnly,
    DiagnosticCloudResume,
    DiagnosticSessionTrail,
    DeprecatedLegacyOpen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum IslandActionDisposition {
    AllowedPrimary,
    AllowedSecondary,
    DiagnosticOnly,
    DeprecatedBlocked,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct IslandRouteInventoryItem {
    pub route_name: &'static str,
    pub current_handler: &'static str,
    pub kind: IslandRouteKind,
    pub disposition: IslandActionDisposition,
    pub allowed_in_primary_ui: bool,
    pub requires_continue_decision_id: bool,
    pub replacement: &'static str,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct IslandStateInventoryItem {
    pub state_name: &'static str,
    pub classification: IslandActionDisposition,
    pub replacement_copy: &'static str,
    pub requires_continue_decision_id: bool,
    pub notes: &'static str,
}

#[allow(dead_code)]
pub static ISLAND_ROUTE_INVENTORY: &[IslandRouteInventoryItem] = &[
    IslandRouteInventoryItem {
        route_name: "native_action_continue",
        current_handler: "continue_from_island -> get_island_continue_state_for_status",
        kind: IslandRouteKind::PrimaryContinueState,
        disposition: IslandActionDisposition::AllowedPrimary,
        allowed_in_primary_ui: true,
        requires_continue_decision_id: false,
        replacement: "island Continue gateway that returns a Continue decision",
        notes: "Primary island answer path; must stay backed by ContinueDecisionResult.",
    },
    IslandRouteInventoryItem {
        route_name: "native_action_perform_continue_action",
        current_handler: "perform_typed_continue_action_from_island -> perform_island_continue_action_for_status",
        kind: IslandRouteKind::PrimaryContinueOpen,
        disposition: IslandActionDisposition::AllowedPrimary,
        allowed_in_primary_ui: true,
        requires_continue_decision_id: true,
        replacement: "typed IslandActionKind dispatched from island available_actions with continue_decision_id for OpenContinueTarget",
        notes: "Typed P4.05 native envelope; non-open action kinds do not require a decision id at runtime.",
    },
    IslandRouteInventoryItem {
        route_name: "native_action_continue_feedback",
        current_handler: "perform_typed_continue_action_from_island -> record_continue_feedback",
        kind: IslandRouteKind::PrimaryContinueState,
        disposition: IslandActionDisposition::AllowedPrimary,
        allowed_in_primary_ui: true,
        requires_continue_decision_id: true,
        replacement: "record_continue_feedback({ source: island_primary, feedback_kind: rejected|ignored })",
        notes: "Island-origin correction path; stores ids and source only, never raw target text.",
    },
    IslandRouteInventoryItem {
        route_name: "native_action_open_resume_point",
        current_handler: "open_resume_point_from_island -> open_resume_point",
        kind: IslandRouteKind::PrimaryContinueOpen,
        disposition: IslandActionDisposition::AllowedPrimary,
        allowed_in_primary_ui: true,
        requires_continue_decision_id: true,
        replacement: "open_resume_point({ continue_decision_id, strict_continue_target: true })",
        notes: "Primary open path; missing decisions must fall back to Smalltalk, not legacy targets.",
    },
    IslandRouteInventoryItem {
        route_name: "native_action_open_main_window",
        current_handler: "open_main_window",
        kind: IslandRouteKind::PrimaryInspectEvidence,
        disposition: IslandActionDisposition::AllowedPrimary,
        allowed_in_primary_ui: true,
        requires_continue_decision_id: false,
        replacement: "Open Smalltalk / Inspect evidence",
        notes: "Safe fallback when Continue has no reliable open target.",
    },
    IslandRouteInventoryItem {
        route_name: "native_action_start_capture",
        current_handler: "start_capture_from_island",
        kind: IslandRouteKind::PrimaryLocalMemoryControl,
        disposition: IslandActionDisposition::AllowedPrimary,
        allowed_in_primary_ui: true,
        requires_continue_decision_id: false,
        replacement: "Start local memory",
        notes: "Allowed as a local-memory control, not as a target selector.",
    },
    IslandRouteInventoryItem {
        route_name: "native_action_stop_capture",
        current_handler: "stop_capture_from_island",
        kind: IslandRouteKind::SecondaryLocalMemoryControl,
        disposition: IslandActionDisposition::AllowedSecondary,
        allowed_in_primary_ui: false,
        requires_continue_decision_id: false,
        replacement: "Pause local memory",
        notes: "Secondary control; stopping capture must not be a prerequisite for Continue.",
    },
    IslandRouteInventoryItem {
        route_name: "native_action_capture_once",
        current_handler: "capture_once_from_island",
        kind: IslandRouteKind::SecondaryLocalMemoryControl,
        disposition: IslandActionDisposition::AllowedSecondary,
        allowed_in_primary_ui: false,
        requires_continue_decision_id: false,
        replacement: "Capture evidence now",
        notes: "Secondary privacy-safe evidence action.",
    },
    IslandRouteInventoryItem {
        route_name: "native_action_show_trail",
        current_handler: "open_main_window",
        kind: IslandRouteKind::PrimaryInspectEvidence,
        disposition: IslandActionDisposition::AllowedSecondary,
        allowed_in_primary_ui: false,
        requires_continue_decision_id: false,
        replacement: "Inspect evidence / Open Smalltalk",
        notes: "Legacy action name; implementation only opens Smalltalk.",
    },
    IslandRouteInventoryItem {
        route_name: "native_action_reconstruct_trail",
        current_handler: "continue_from_island",
        kind: IslandRouteKind::DeprecatedLegacyOpen,
        disposition: IslandActionDisposition::DeprecatedBlocked,
        allowed_in_primary_ui: false,
        requires_continue_decision_id: false,
        replacement: "native_action_continue",
        notes: "Legacy action alias; it currently routes to Continue but must not return as primary copy.",
    },
    IslandRouteInventoryItem {
        route_name: "native_action_resume_me",
        current_handler: "open_resume_point_from_island",
        kind: IslandRouteKind::DeprecatedLegacyOpen,
        disposition: IslandActionDisposition::DeprecatedBlocked,
        allowed_in_primary_ui: false,
        requires_continue_decision_id: true,
        replacement: "native_action_open_resume_point",
        notes: "Legacy action alias; the Continue-first primary action is open_resume_point with a decision id.",
    },
    IslandRouteInventoryItem {
        route_name: "native_action_toggle_expanded",
        current_handler: "toggle_expanded_from_native",
        kind: IslandRouteKind::PresentationOnly,
        disposition: IslandActionDisposition::AllowedSecondary,
        allowed_in_primary_ui: false,
        requires_continue_decision_id: false,
        replacement: "Presentation toggle",
        notes: "Panel presentation only; it must not choose or open targets.",
    },
    IslandRouteInventoryItem {
        route_name: "native_action_collapse",
        current_handler: "set_session_island_expanded(false)",
        kind: IslandRouteKind::PresentationOnly,
        disposition: IslandActionDisposition::AllowedSecondary,
        allowed_in_primary_ui: false,
        requires_continue_decision_id: false,
        replacement: "Presentation collapse",
        notes: "Panel presentation only; it must not choose or open targets.",
    },
    IslandRouteInventoryItem {
        route_name: "apply_continue_decision_to_snapshot",
        current_handler: "ContinueDecisionResult -> SessionIslandSnapshot",
        kind: IslandRouteKind::PrimaryContinueState,
        disposition: IslandActionDisposition::AllowedPrimary,
        allowed_in_primary_ui: true,
        requires_continue_decision_id: true,
        replacement: "IslandContinueState derived from ContinueDecisionResult",
        notes: "Compatibility adapter; canonical island state is the gateway DTO.",
    },
    IslandRouteInventoryItem {
        route_name: "apply_cloud_resume_to_snapshot",
        current_handler: "CloudResumeResult -> SessionIslandSnapshot",
        kind: IslandRouteKind::DiagnosticCloudResume,
        disposition: IslandActionDisposition::DiagnosticOnly,
        allowed_in_primary_ui: false,
        requires_continue_decision_id: false,
        replacement: "ContinueDecisionResult -> SessionIslandSnapshot",
        notes: "Legacy diagnostic helper; not called by the current island primary path.",
    },
    IslandRouteInventoryItem {
        route_name: "remember_cloud_resume_output_path",
        current_handler: "CloudResumeResult.output_path cache",
        kind: IslandRouteKind::DiagnosticCloudResume,
        disposition: IslandActionDisposition::DiagnosticOnly,
        allowed_in_primary_ui: false,
        requires_continue_decision_id: false,
        replacement: "continue_decision_id cache",
        notes: "Legacy diagnostic cache; primary island open must not read this.",
    },
    IslandRouteInventoryItem {
        route_name: "legacy_command_run_cloud_resume",
        current_handler: "capture::run_cloud_resume Tauri command",
        kind: IslandRouteKind::DiagnosticCloudResume,
        disposition: IslandActionDisposition::DiagnosticOnly,
        allowed_in_primary_ui: false,
        requires_continue_decision_id: false,
        replacement: "get_continue_decision",
        notes: "Tauri command remains outside primary island behavior.",
    },
    IslandRouteInventoryItem {
        route_name: "legacy_command_build_resume_query_bundle",
        current_handler: "capture::build_resume_query_bundle Tauri command",
        kind: IslandRouteKind::DiagnosticSessionTrail,
        disposition: IslandActionDisposition::DiagnosticOnly,
        allowed_in_primary_ui: false,
        requires_continue_decision_id: false,
        replacement: "get_continue_decision_trace or Continue evidence inspection",
        notes: "Stop-time/export diagnostic path; not an island target selector.",
    },
    IslandRouteInventoryItem {
        route_name: "legacy_command_get_native_resume_card",
        current_handler: "capture::get_native_resume_card Tauri command",
        kind: IslandRouteKind::DiagnosticSessionTrail,
        disposition: IslandActionDisposition::DiagnosticOnly,
        allowed_in_primary_ui: false,
        requires_continue_decision_id: false,
        replacement: "ContinueDecisionResult handoff fields",
        notes: "Native card diagnostics must not feed island primary state.",
    },
    IslandRouteInventoryItem {
        route_name: "legacy_command_get_native_storyboard_dossier",
        current_handler: "capture::get_native_storyboard_dossier Tauri command",
        kind: IslandRouteKind::DiagnosticSessionTrail,
        disposition: IslandActionDisposition::DiagnosticOnly,
        allowed_in_primary_ui: false,
        requires_continue_decision_id: false,
        replacement: "Continue evidence inspection",
        notes: "Storyboard diagnostics must not feed island primary state.",
    },
    IslandRouteInventoryItem {
        route_name: "deprecated_open_without_continue_decision_id",
        current_handler: "open_resume_point_from_island missing decision fallback",
        kind: IslandRouteKind::DeprecatedLegacyOpen,
        disposition: IslandActionDisposition::DeprecatedBlocked,
        allowed_in_primary_ui: false,
        requires_continue_decision_id: true,
        replacement: "refresh Continue first, then strict open by continue_decision_id",
        notes: "Blocked behavior; current code opens Smalltalk when no decision can be obtained.",
    },
];

#[allow(dead_code)]
pub static ISLAND_STATE_INVENTORY: &[IslandStateInventoryItem] = &[
    IslandStateInventoryItem {
        state_name: "hidden",
        classification: IslandActionDisposition::AllowedSecondary,
        replacement_copy: "Hidden",
        requires_continue_decision_id: false,
        notes: "No product copy is visible.",
    },
    IslandStateInventoryItem {
        state_name: "ready",
        classification: IslandActionDisposition::AllowedPrimary,
        replacement_copy: "Continue",
        requires_continue_decision_id: false,
        notes: "Idle state should invite Continue or local memory, not session resume.",
    },
    IslandStateInventoryItem {
        state_name: "starting",
        classification: IslandActionDisposition::AllowedPrimary,
        replacement_copy: "Starting local memory",
        requires_continue_decision_id: false,
        notes: "Local-memory status only.",
    },
    IslandStateInventoryItem {
        state_name: "recording_compact",
        classification: IslandActionDisposition::AllowedPrimary,
        replacement_copy: "Local memory active",
        requires_continue_decision_id: false,
        notes: "Status only; it must not imply recorder-first product behavior.",
    },
    IslandStateInventoryItem {
        state_name: "recording_expanded",
        classification: IslandActionDisposition::AllowedPrimary,
        replacement_copy: "Local memory active",
        requires_continue_decision_id: false,
        notes: "Status only; it must not imply recorder-first product behavior.",
    },
    IslandStateInventoryItem {
        state_name: "processing",
        classification: IslandActionDisposition::AllowedPrimary,
        replacement_copy: "Checking Continue",
        requires_continue_decision_id: false,
        notes: "Busy state; future P4 steps should avoid pause-session wording.",
    },
    IslandStateInventoryItem {
        state_name: "stopped_toast",
        classification: IslandActionDisposition::AllowedSecondary,
        replacement_copy: "Local memory paused",
        requires_continue_decision_id: false,
        notes: "Transient status only; Continue must not require stopping memory.",
    },
    IslandStateInventoryItem {
        state_name: "trail_reconstructing",
        classification: IslandActionDisposition::DeprecatedBlocked,
        replacement_copy: "Checking Continue",
        requires_continue_decision_id: false,
        notes: "Legacy state name; visible copy should be Continue-first.",
    },
    IslandStateInventoryItem {
        state_name: "resume_ready",
        classification: IslandActionDisposition::AllowedPrimary,
        replacement_copy: "Continue ready",
        requires_continue_decision_id: true,
        notes: "Must only be shown as a primary state when backed by continue_decision_id.",
    },
    IslandStateInventoryItem {
        state_name: "error",
        classification: IslandActionDisposition::AllowedPrimary,
        replacement_copy: "Continue unavailable",
        requires_continue_decision_id: false,
        notes: "No target open should be offered.",
    },
];

#[derive(Debug, Deserialize)]
struct SessionIslandAction {
    action: SessionIslandActionKind,
    action_kind: Option<IslandActionKind>,
    decision_id: Option<String>,
    source: Option<String>,
    trace_id: Option<String>,
    task_snapshot_id: Option<String>,
    task_snapshot_revision: Option<i64>,
    affected_task_field: Option<String>,
    task_hypothesis_id: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SessionIslandActionKind {
    Continue,
    StartCapture,
    StopCapture,
    CaptureOnce,
    ReconstructTrail,
    ShowTrail,
    OpenResumePoint,
    PerformContinueAction,
    OpenMainWindow,
    ResumeMe,
    ToggleExpanded,
    Collapse,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IslandContinueActionInput {
    pub action_kind: IslandActionKind,
    pub decision_id: Option<String>,
    pub source: Option<String>,
    pub trace_id: Option<String>,
    pub task_snapshot_id: Option<String>,
    pub task_snapshot_revision: Option<i64>,
    pub affected_task_field: Option<String>,
    pub task_hypothesis_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IslandContinueActionResult {
    pub action_kind: IslandActionKind,
    pub decision_id: Option<String>,
    pub opened: bool,
    pub open_strategy: Option<String>,
    pub refreshed_state: Option<IslandContinueState>,
    pub warnings: Vec<String>,
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
            island_continue_state: None,
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
            island_continue_state: Some(IslandContinueState::error(
                now_millis(),
                Some("island_continue_error".to_string()),
            )),
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

#[allow(clippy::too_many_arguments)]
pub(super) fn write_island_continue_audit(
    audit_path: Option<&str>,
    state: &IslandContinueState,
    trigger_reason: &str,
    source: &str,
    open_attempted: bool,
    open_allowed: bool,
    open_blocked_reason: Option<&str>,
) {
    let Some(audit_path) = audit_path.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    let output_dir = std::path::Path::new(audit_path);
    let decision_dir = output_dir.join("decision");
    if std::fs::create_dir_all(&decision_dir).is_err() {
        return;
    }
    let available_actions = state
        .available_actions
        .iter()
        .filter(|action| action.enabled)
        .map(|action| {
            serde_json::to_value(&action.kind)
                .ok()
                .and_then(|value| value.as_str().map(str::to_string))
                .unwrap_or_else(|| "unknown".to_string())
        })
        .collect::<Vec<_>>();
    let display_state = serde_json::to_value(&state.display_state)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "error".to_string());
    let payload = serde_json::json!({
        "schema": "smalltalk.island_continue_audit.v1",
        "island": {
            "state_schema": state.schema,
            "trigger_reason": trigger_reason,
            "decision_id": state.decision_id,
            "display_state": display_state,
            "available_actions": available_actions,
            "open_attempted": open_attempted,
            "open_allowed": open_allowed,
            "open_blocked_reason": open_blocked_reason,
            "source": source,
            "decision_cache_hit": state.decision_cache_hit,
            "decision_stale": state.decision_stale,
            "validation_status": state.validation_status,
            "suppression_reasons": state.suppression_reasons,
            "warnings": state.warnings,
        }
    });
    if let Ok(json) = serde_json::to_string_pretty(&payload) {
        let _ = std::fs::write(decision_dir.join("island_continue_audit.json"), json);
    }
}

pub fn update_session_island_from_status(status: &CaptureStatus, state: SessionIslandState) {
    update_session_island(snapshot_from_status(status, state));
}

#[tauri::command]
pub fn get_island_continue_state(
    app: AppHandle,
    state: tauri::State<crate::capture::CaptureState>,
    input: IslandContinueStateInput,
) -> Result<IslandContinueState, String> {
    gateway::get_island_continue_state(app, state, input)
}

#[tauri::command]
pub fn perform_island_continue_action(
    app: AppHandle,
    state: tauri::State<crate::capture::CaptureState>,
    input: IslandContinueActionInput,
) -> Result<IslandContinueActionResult, String> {
    let status = crate::capture::capture_status(app.clone(), state)?;
    perform_island_continue_action_for_status(app, status, input)
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
        island_continue_state: None,
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
    status: &CaptureStatus,
) {
    if matches!(
        snapshot.state,
        SessionIslandState::Hidden
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
    let session_changed = remembered.session_id.as_deref() != status_session_id
        && (remembered.session_id.is_some() || status_session_id.is_some());

    let latest_capture_at = snapshot.last_capture_at_ms.unwrap_or_default();
    let remembered_evidence_at = remembered.evidence_updated_at_ms.unwrap_or_default();
    let has_new_evidence = session_changed
        || snapshot.frame_count > remembered.frame_count
        || snapshot.trail_moment_count > remembered.signal_count
        || snapshot.event_count > remembered.event_count
        || latest_capture_at > remembered_evidence_at;

    snapshot.continue_decision_id = Some(remembered.decision_id);
    snapshot.continue_openable = Some(remembered.continue_openable);
    snapshot.decision_updated_at_ms = remembered.decision_updated_at_ms;
    snapshot.evidence_updated_at_ms = Some(latest_capture_at.max(remembered_evidence_at));
    snapshot.resume_point = remembered.resume_point.clone();
    snapshot.island_continue_state = Some(remembered.island_continue_state.clone());

    if has_new_evidence {
        if let Some(state) = snapshot.island_continue_state.as_mut() {
            state.display_state = IslandDisplayState::NeedsRefresh;
            state.decision_stale = true;
            state.evidence_watermark_ms = Some(latest_capture_at.max(remembered_evidence_at));
            state.available_actions = vec![
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
            ];
        }
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
        SessionIslandActionKind::OpenResumePoint => {
            open_resume_point_from_island(action.decision_id, action.source, action.trace_id)
        }
        SessionIslandActionKind::PerformContinueAction => {
            perform_typed_continue_action_from_island(
                action.action_kind,
                action.decision_id,
                action.source,
                action.trace_id,
                action.task_snapshot_id,
                action.task_snapshot_revision,
                action.affected_task_field,
                action.task_hypothesis_id,
            )
        }
        SessionIslandActionKind::OpenMainWindow => open_main_window(),
        SessionIslandActionKind::ResumeMe => {
            eprintln!("[session_island] blocked deprecated resume_me open action");
            open_main_window();
        }
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
        let state = app.state::<crate::capture::CaptureState>();
        let next_status = crate::capture::capture_status(app.clone(), state).unwrap_or_else(|_| {
            crate::capture::CaptureStatus {
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
            }
        });
        match gateway::get_island_continue_state_for_status(
            app.clone(),
            next_status.clone(),
            IslandContinueStateInput::for_user_continue(remembered_continue_decision_id()),
        ) {
            Ok(gateway_result) => {
                let mut snapshot =
                    snapshot_from_status(&next_status, SessionIslandState::ResumeReady);
                if let Some(decision) = gateway_result.decision.as_ref() {
                    apply_continue_decision_to_snapshot(&mut snapshot, decision);
                    let _ = app.emit("smalltalk-continue-updated", decision.clone());
                } else {
                    apply_island_continue_state_to_snapshot(&mut snapshot, &gateway_result.state);
                }
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

fn island_continue_decision_request() -> crate::continuation::ContinueDecisionRequest {
    crate::continuation::ContinueDecisionRequest {
        mode: Some("normal".to_string()),
        rebuild_layers: Some(false),
        micro_inference_enabled: Some(true),
        activity_recap_model_enabled: Some(true),
        max_candidates_for_model: Some(5),
        audit_output_enabled: Some(false),
        audit_mode: Some(crate::continuation::ContinueAuditMode::None),
        request_trigger: Some("island".to_string()),
        ..Default::default()
    }
}

fn apply_continue_decision_to_snapshot(
    snapshot: &mut SessionIslandSnapshot,
    decision: &crate::continuation::ContinueDecisionResult,
) {
    let decision_updated_at_ms = now_millis();
    let freshness = IslandFreshness {
        evidence_watermark_ms: decision_evidence_updated_at_ms(decision)
            .or(snapshot.last_capture_at_ms),
        newest_evidence_ms: snapshot.last_capture_at_ms,
        decision_updated_at_ms: Some(decision_updated_at_ms),
        decision_stale: false,
    };
    let context = IslandStateContext {
        local_memory_running: matches!(
            snapshot.state,
            SessionIslandState::RecordingCompact | SessionIslandState::RecordingExpanded
        ),
        has_local_memory: snapshot.frame_count > 0
            || snapshot.event_count > 0
            || snapshot.trail_moment_count > 0,
    };
    let island_state = island_state_from_continue_decision(decision, freshness, context);
    let continue_openable = island_state.allows_open_continue_target();
    let continue_freshness = continue_freshness_from_island_state(&island_state);

    snapshot.continue_decision_id = Some(decision.decision_id.clone());
    snapshot.continue_freshness = Some(continue_freshness);
    snapshot.decision_updated_at_ms = Some(decision_updated_at_ms);
    snapshot.evidence_updated_at_ms = island_state
        .evidence_watermark_ms
        .or(snapshot.last_capture_at_ms)
        .or(snapshot.decision_updated_at_ms);
    snapshot.continue_openable = Some(continue_openable);
    snapshot.resume_source = Some("continue".to_string());
    snapshot.resume_model = None;
    snapshot.resume_response_id = None;
    snapshot.resume_headline = Some(headline_from_island_state(&island_state).to_string());
    snapshot.resume_detail = island_state.next_action.clone();
    snapshot.resume_point = island_state
        .resume_work_target
        .as_ref()
        .or(island_state.return_target.as_ref())
        .and_then(|target| clean_one_line(Some(&target.title)));
    snapshot.resume_warning = island_state
        .missing_evidence
        .first()
        .or_else(|| island_state.warnings.first())
        .and_then(|warning| clean_one_line(Some(warning)));
    snapshot.island_continue_state = Some(island_state.clone());
    remember_continue_decision_from_snapshot(
        decision,
        snapshot,
        snapshot.continue_freshness.clone().unwrap_or_default(),
        continue_openable,
        island_state,
    );
}

fn apply_island_continue_state_to_snapshot(
    snapshot: &mut SessionIslandSnapshot,
    island_state: &IslandContinueState,
) {
    let continue_openable = island_state.allows_open_continue_target();
    snapshot.continue_decision_id = island_state.decision_id.clone();
    snapshot.continue_freshness = Some(continue_freshness_from_island_state(island_state));
    snapshot.decision_updated_at_ms = Some(island_state.generated_at_ms);
    snapshot.evidence_updated_at_ms = island_state
        .evidence_watermark_ms
        .or(snapshot.last_capture_at_ms)
        .or(snapshot.decision_updated_at_ms);
    snapshot.continue_openable = Some(continue_openable);
    snapshot.resume_source = Some("continue".to_string());
    snapshot.resume_model = None;
    snapshot.resume_response_id = None;
    snapshot.resume_headline = Some(headline_from_island_state(island_state).to_string());
    snapshot.resume_detail = island_state.next_action.clone();
    snapshot.resume_point = island_state
        .resume_work_target
        .as_ref()
        .or(island_state.return_target.as_ref())
        .and_then(|target| clean_one_line(Some(&target.title)));
    snapshot.resume_warning = island_state
        .missing_evidence
        .first()
        .or_else(|| island_state.warnings.first())
        .and_then(|warning| clean_one_line(Some(warning)));
    snapshot.island_continue_state = Some(island_state.clone());
}

fn continue_freshness_from_island_state(state: &IslandContinueState) -> String {
    match state.display_state {
        IslandDisplayState::ContinueReady => "current",
        IslandDisplayState::NeedsRefresh => "new_evidence",
        IslandDisplayState::CheckingContinue => "updating",
        IslandDisplayState::NoLocalMemory | IslandDisplayState::LocalMemoryWarming => {
            "needs_evidence"
        }
        IslandDisplayState::ThinCurrentWork
        | IslandDisplayState::TargetSuppressed
        | IslandDisplayState::SupportBlocked
        | IslandDisplayState::InspectOnly
        | IslandDisplayState::NoClearContinuation
        | IslandDisplayState::Error => "thin_evidence",
    }
    .to_string()
}

fn headline_from_island_state(state: &IslandContinueState) -> &'static str {
    match state.display_state {
        IslandDisplayState::ContinueReady => "Ready to continue",
        IslandDisplayState::ThinCurrentWork => "Evidence is thin",
        IslandDisplayState::TargetSuppressed => "Target suppressed",
        IslandDisplayState::SupportBlocked => "Support branch blocked",
        IslandDisplayState::NeedsRefresh => "New evidence",
        IslandDisplayState::NoClearContinuation => "No clear continuation",
        IslandDisplayState::InspectOnly => "Inspect evidence",
        IslandDisplayState::NoLocalMemory => "No local memory yet",
        IslandDisplayState::LocalMemoryWarming => "Local memory warming",
        IslandDisplayState::CheckingContinue => "Checking Continue",
        IslandDisplayState::Error => "Continue unavailable",
    }
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

fn open_resume_point_from_island(
    decision_id: Option<String>,
    source: Option<String>,
    trace_id: Option<String>,
) {
    let Some(app) = APP_HANDLE.get().cloned() else {
        eprintln!("[session_island] open resume point requested before AppHandle was ready");
        return;
    };
    thread::spawn(move || {
        let state = app.state::<crate::capture::CaptureState>();
        let status = match crate::capture::capture_status(app.clone(), state) {
            Ok(status) => status,
            Err(error) => {
                eprintln!(
                    "[session_island] capture_status failed before open: {}",
                    error
                );
                update_session_island(SessionIslandSnapshot::error(
                    "Continue is not ready yet. Open Smalltalk to inspect local evidence."
                        .to_string(),
                ));
                open_main_window();
                return;
            }
        };
        let result = perform_island_continue_action_for_status(
            app.clone(),
            status,
            IslandContinueActionInput {
                action_kind: IslandActionKind::OpenContinueTarget,
                decision_id,
                source: source.or_else(|| Some("native_callback".to_string())),
                trace_id,
                task_snapshot_id: None,
                task_snapshot_revision: None,
                affected_task_field: None,
                task_hypothesis_id: None,
            },
        );
        match result {
            Ok(result) => {
                if !result.warnings.is_empty() {
                    eprintln!(
                        "[session_island] island Continue action warnings: {}",
                        result.warnings.join(" | ")
                    );
                }
                if !result.opened {
                    open_main_window();
                }
            }
            Err(error) => {
                eprintln!("[session_island] island Continue action failed: {}", error);
                open_main_window();
            }
        }
    });
}

fn perform_typed_continue_action_from_island(
    action_kind: Option<IslandActionKind>,
    decision_id: Option<String>,
    source: Option<String>,
    trace_id: Option<String>,
    task_snapshot_id: Option<String>,
    task_snapshot_revision: Option<i64>,
    affected_task_field: Option<String>,
    task_hypothesis_id: Option<String>,
) {
    let Some(app) = APP_HANDLE.get().cloned() else {
        eprintln!("[session_island] typed Continue action requested before AppHandle was ready");
        return;
    };
    let Some(action_kind) = action_kind else {
        eprintln!("[session_island] typed Continue action missing action_kind");
        open_main_window();
        return;
    };

    thread::spawn(move || {
        let state = app.state::<crate::capture::CaptureState>();
        let status = match crate::capture::capture_status(app.clone(), state) {
            Ok(status) => status,
            Err(error) => {
                eprintln!(
                    "[session_island] capture_status failed before typed Continue action: {}",
                    error
                );
                update_session_island(SessionIslandSnapshot::error(
                    "Continue is not ready yet. Open Smalltalk to inspect local evidence."
                        .to_string(),
                ));
                open_main_window();
                return;
            }
        };
        let opens_target = matches!(action_kind, IslandActionKind::OpenContinueTarget);
        let result = perform_island_continue_action_for_status(
            app.clone(),
            status.clone(),
            IslandContinueActionInput {
                action_kind,
                decision_id,
                source: source.or_else(|| Some("native_callback".to_string())),
                trace_id,
                task_snapshot_id,
                task_snapshot_revision,
                affected_task_field,
                task_hypothesis_id,
            },
        );
        match result {
            Ok(result) => {
                if let Some(refreshed_state) = result.refreshed_state.as_ref() {
                    let mut snapshot =
                        snapshot_from_status(&status, SessionIslandState::ResumeReady);
                    apply_island_continue_state_to_snapshot(&mut snapshot, refreshed_state);
                    update_session_island(snapshot);
                    show_session_island();
                }
                if !result.warnings.is_empty() {
                    eprintln!(
                        "[session_island] typed island Continue action warnings: {}",
                        result.warnings.join(" | ")
                    );
                }
                if opens_target && !result.opened {
                    open_main_window();
                }
            }
            Err(error) => {
                eprintln!(
                    "[session_island] typed island Continue action failed: {}",
                    error
                );
                open_main_window();
            }
        }
    });
}

fn perform_island_continue_action_for_status(
    app: AppHandle,
    status: CaptureStatus,
    input: IslandContinueActionInput,
) -> Result<IslandContinueActionResult, String> {
    match input.action_kind {
        IslandActionKind::RefreshContinue => {
            let result = gateway::get_island_continue_state_for_status(
                app,
                status,
                IslandContinueStateInput::for_user_continue(input.decision_id.clone()),
            )?;
            Ok(IslandContinueActionResult {
                action_kind: IslandActionKind::RefreshContinue,
                decision_id: result.state.decision_id.clone(),
                opened: false,
                open_strategy: None,
                refreshed_state: Some(result.state),
                warnings: Vec::new(),
            })
        }
        IslandActionKind::OpenContinueTarget => {
            open_continue_target_from_island(app, status, input)
        }
        IslandActionKind::MarkWrongTarget => {
            record_island_continue_feedback(app, status, input, "rejected", "island_wrong_target")
        }
        IslandActionKind::MarkNotUseful => {
            record_island_continue_feedback(app, status, input, "ignored", "island_not_useful")
        }
        IslandActionKind::ChooseTaskAlternative
        | IslandActionKind::RejectSelectedTask
        | IslandActionKind::RejectTaskAlternative
        | IslandActionKind::MarkSupportingWork
        | IslandActionKind::MarkUnrelatedActivity
        | IslandActionKind::MarkTaskCompleted
        | IslandActionKind::ReactivateTask => {
            record_scoped_island_task_feedback(app, status, input)
        }
        IslandActionKind::InspectEvidence | IslandActionKind::OpenSmalltalk => {
            open_main_window();
            Ok(IslandContinueActionResult {
                action_kind: input.action_kind,
                decision_id: input.decision_id,
                opened: false,
                open_strategy: Some("open_smalltalk".to_string()),
                refreshed_state: None,
                warnings: Vec::new(),
            })
        }
        IslandActionKind::StartLocalMemory => {
            start_capture_from_island();
            Ok(IslandContinueActionResult {
                action_kind: IslandActionKind::StartLocalMemory,
                decision_id: input.decision_id,
                opened: false,
                open_strategy: Some("start_local_memory".to_string()),
                refreshed_state: None,
                warnings: Vec::new(),
            })
        }
        IslandActionKind::CaptureEvidenceNow => {
            capture_once_from_island();
            Ok(IslandContinueActionResult {
                action_kind: IslandActionKind::CaptureEvidenceNow,
                decision_id: input.decision_id,
                opened: false,
                open_strategy: Some("capture_evidence_now".to_string()),
                refreshed_state: None,
                warnings: Vec::new(),
            })
        }
    }
}

fn record_island_continue_feedback(
    app: AppHandle,
    status: CaptureStatus,
    input: IslandContinueActionInput,
    feedback_kind: &str,
    warning_code: &str,
) -> Result<IslandContinueActionResult, String> {
    let decision_id = input
        .decision_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let feedback = crate::capture::record_continue_feedback(
        app.clone(),
        crate::continuation::ContinueExplicitFeedbackRequest {
            decision_id: decision_id.clone(),
            selected_candidate_id: None,
            workstream_id: None,
            target_artifact_id: None,
            corrected_artifact_id: None,
            feedback_kind: feedback_kind.to_string(),
            note: None,
            source: Some("island_primary".to_string()),
            task_snapshot_id: None,
            task_snapshot_revision: None,
            affected_task_field: None,
            task_hypothesis_id: None,
        },
    )?;
    let refreshed = gateway::get_island_continue_state_for_status(
        app,
        status,
        IslandContinueStateInput {
            reason: IslandContinueReason::EvidenceChanged,
            existing_decision_id: decision_id.clone(),
            allow_refresh: true,
            force_refresh: true,
            source: Some("island_primary".to_string()),
        },
    )?
    .state;
    Ok(IslandContinueActionResult {
        action_kind: input.action_kind,
        decision_id: decision_id.or(feedback.decision_id),
        opened: false,
        open_strategy: Some("feedback_recorded".to_string()),
        refreshed_state: Some(refreshed),
        warnings: vec![warning_code.to_string()],
    })
}

fn scoped_task_feedback_contract(
    kind: &IslandActionKind,
) -> Option<(&'static str, &'static str, bool)> {
    match kind {
        IslandActionKind::ChooseTaskAlternative => Some(("corrected", "hypothesis", true)),
        IslandActionKind::RejectSelectedTask => Some(("rejected", "task_summary", false)),
        IslandActionKind::RejectTaskAlternative => Some(("rejected", "hypothesis", true)),
        IslandActionKind::MarkSupportingWork => Some(("supporting_work", "relationship", true)),
        IslandActionKind::MarkUnrelatedActivity => {
            Some(("unrelated_activity", "relationship", true))
        }
        IslandActionKind::MarkTaskCompleted => Some(("completed", "task_status", true)),
        IslandActionKind::ReactivateTask => Some(("reactivated", "task_status", true)),
        _ => None,
    }
}

fn record_scoped_island_task_feedback(
    app: AppHandle,
    status: CaptureStatus,
    input: IslandContinueActionInput,
) -> Result<IslandContinueActionResult, String> {
    let (feedback_kind, expected_field, hypothesis_required) =
        scoped_task_feedback_contract(&input.action_kind)
            .ok_or_else(|| "island action is not scoped task feedback".to_string())?;
    let snapshot_id = input
        .task_snapshot_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "scoped island task feedback requires snapshot id".to_string())?;
    let snapshot_revision = input
        .task_snapshot_revision
        .filter(|revision| *revision > 0)
        .ok_or_else(|| "scoped island task feedback requires snapshot revision".to_string())?;
    if input.affected_task_field.as_deref() != Some(expected_field) {
        return Err("scoped island task feedback field does not match its action".into());
    }
    if hypothesis_required
        && input
            .task_hypothesis_id
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err("scoped island task feedback requires hypothesis id".into());
    }
    let decision_id = input
        .decision_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let feedback = crate::capture::record_continue_feedback(
        app.clone(),
        crate::continuation::ContinueExplicitFeedbackRequest {
            decision_id: decision_id.clone(),
            selected_candidate_id: None,
            workstream_id: None,
            target_artifact_id: None,
            corrected_artifact_id: None,
            feedback_kind: feedback_kind.to_string(),
            note: None,
            source: Some("island_primary".to_string()),
            task_snapshot_id: Some(snapshot_id.to_string()),
            task_snapshot_revision: Some(snapshot_revision),
            affected_task_field: Some(expected_field.to_string()),
            task_hypothesis_id: input.task_hypothesis_id.clone(),
        },
    )?;
    let refreshed = gateway::get_island_continue_state_for_status(
        app,
        status,
        IslandContinueStateInput {
            reason: IslandContinueReason::EvidenceChanged,
            existing_decision_id: decision_id.clone(),
            allow_refresh: true,
            force_refresh: true,
            source: Some("island_primary".to_string()),
        },
    )?
    .state;
    Ok(IslandContinueActionResult {
        action_kind: input.action_kind,
        decision_id: decision_id.or(feedback.decision_id),
        opened: false,
        open_strategy: Some("scoped_task_feedback_recorded".to_string()),
        refreshed_state: Some(refreshed),
        warnings: Vec::new(),
    })
}

fn open_continue_target_from_island(
    app: AppHandle,
    status: CaptureStatus,
    input: IslandContinueActionInput,
) -> Result<IslandContinueActionResult, String> {
    let _native_source = input.source.as_deref();
    let _trace_id = input.trace_id.as_deref();
    let mut warnings = Vec::new();
    let Some(decision_id) = input
        .decision_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
    else {
        warnings.push(
            "blocked_by_continue_policy:island_primary_requires_continue_decision_id".to_string(),
        );
        return Ok(IslandContinueActionResult {
            action_kind: IslandActionKind::OpenContinueTarget,
            decision_id: None,
            opened: false,
            open_strategy: Some("blocked_by_continue_policy".to_string()),
            refreshed_state: None,
            warnings,
        });
    };

    let gateway_result = gateway::get_island_continue_state_for_status(
        app.clone(),
        status.clone(),
        IslandContinueStateInput {
            reason: IslandContinueReason::UserPressedContinue,
            existing_decision_id: Some(decision_id.clone()),
            allow_refresh: false,
            force_refresh: false,
            source: Some("island_primary".to_string()),
        },
    )?;

    let state_decision_matches = gateway_result.state.decision_id.as_deref() == Some(&decision_id);
    if !state_decision_matches
        || gateway_result.state.decision_stale
        || !gateway_result.state.allows_open_continue_target()
    {
        let refreshed_state = if gateway_result.state.decision_stale
            || matches!(
                gateway_result.state.display_state,
                IslandDisplayState::NeedsRefresh
            ) {
            let refreshed = gateway::get_island_continue_state_for_status(
                app.clone(),
                status.clone(),
                IslandContinueStateInput::for_user_continue(Some(decision_id.clone())),
            )?
            .state;
            Some(refreshed)
        } else {
            Some(gateway_result.state)
        };
        if let Some(state) = refreshed_state.as_ref() {
            let mut snapshot = snapshot_from_status(&status, SessionIslandState::ResumeReady);
            apply_island_continue_state_to_snapshot(&mut snapshot, state);
            update_session_island(snapshot);
            write_island_continue_audit(
                state.audit_path.as_deref(),
                state,
                "user_pressed_continue",
                "island_primary",
                true,
                false,
                Some(if state.decision_stale {
                    "stale_decision"
                } else {
                    match state.display_state {
                        IslandDisplayState::TargetSuppressed => "p1_suppressed",
                        IslandDisplayState::SupportBlocked => "p2_support_blocked",
                        IslandDisplayState::ThinCurrentWork | IslandDisplayState::InspectOnly => {
                            "thin_weak_surface"
                        }
                        _ => "state_not_openable",
                    }
                }),
            );
        }
        warnings.push("blocked_by_continue_policy:island_primary_state_not_openable".to_string());
        return Ok(IslandContinueActionResult {
            action_kind: IslandActionKind::OpenContinueTarget,
            decision_id: Some(decision_id),
            opened: false,
            open_strategy: Some("blocked_by_continue_policy".to_string()),
            refreshed_state,
            warnings,
        });
    }

    let result = crate::capture::open_resume_point(
        app,
        Some(OpenResumePointInput {
            output_path: None,
            session_id: None,
            continue_decision_id: Some(decision_id.clone()),
            target_artifact_id: None,
            source: Some("island_primary".to_string()),
            diagnostic_allowed: Some(false),
            strict_continue_target: true,
            current_frame_id: None,
            target_frame_id: None,
        }),
    )?;
    let opened = result.opened_url.is_some()
        && !result.strategy.starts_with("smalltalk_")
        && result.strategy != "blocked_by_continue_policy";
    warnings.extend(result.warnings);
    write_island_continue_audit(
        gateway_result.state.audit_path.as_deref(),
        &gateway_result.state,
        "user_pressed_continue",
        "island_primary",
        true,
        opened,
        if opened {
            None
        } else {
            Some("continue_open_fallback")
        },
    );
    Ok(IslandContinueActionResult {
        action_kind: IslandActionKind::OpenContinueTarget,
        decision_id: Some(decision_id),
        opened,
        open_strategy: Some(result.strategy),
        refreshed_state: Some(gateway_result.state),
        warnings,
    })
}

fn remember_continue_decision(decision: &crate::continuation::ContinueDecisionResult) {
    remember_continue_decision_id(&decision.decision_id);
}

fn remember_continue_decision_from_snapshot(
    decision: &crate::continuation::ContinueDecisionResult,
    snapshot: &SessionIslandSnapshot,
    continue_freshness: String,
    continue_openable: bool,
    island_continue_state: IslandContinueState,
) {
    remember_continue_decision(decision);
    if let Ok(mut slot) = LAST_CONTINUE_ISLAND_STATE.lock() {
        *slot = Some(RememberedContinueIslandState {
            session_id: snapshot.session_id.clone(),
            decision_id: decision.decision_id.clone(),
            request_trigger: decision.request_trigger.clone(),
            task_turn_id: decision
                .current_task_turn
                .as_ref()
                .map(|turn| turn.task_turn_id.clone()),
            task_turn_revision: decision
                .current_task_turn
                .as_ref()
                .map(|turn| turn.revision),
            task_confidence: decision.confidence_summary.task.score,
            wording_source: decision.wording_source.clone(),
            target_selection_source: decision.target_selection_source.clone(),
            resume_headline: snapshot.resume_headline.clone(),
            resume_detail: snapshot.resume_detail.clone(),
            resume_point: snapshot.resume_point.clone(),
            resume_warning: snapshot.resume_warning.clone(),
            continue_freshness,
            evidence_updated_at_ms: snapshot.evidence_updated_at_ms,
            decision_updated_at_ms: snapshot.decision_updated_at_ms,
            continue_openable,
            feedback_or_open_watermark_ms: None,
            frame_count: snapshot.frame_count,
            signal_count: snapshot.trail_moment_count,
            event_count: snapshot.event_count,
            island_continue_state,
        });
    }
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
        match crate::capture::stop_capture_impl(app.clone(), state.inner()) {
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
    fn native_scoped_task_feedback_envelope_preserves_exact_identity() {
        let action: SessionIslandAction = serde_json::from_value(serde_json::json!({
            "action": "perform_continue_action",
            "action_kind": "choose_task_alternative",
            "decision_id": "decision-a",
            "task_snapshot_id": "snapshot-a",
            "task_snapshot_revision": 4,
            "affected_task_field": "hypothesis",
            "task_hypothesis_id": "hypothesis-b",
            "source": "native_island"
        }))
        .unwrap();
        assert_eq!(
            action.action_kind,
            Some(IslandActionKind::ChooseTaskAlternative)
        );
        assert_eq!(action.task_snapshot_id.as_deref(), Some("snapshot-a"));
        assert_eq!(action.task_snapshot_revision, Some(4));
        assert_eq!(action.affected_task_field.as_deref(), Some("hypothesis"));
        assert_eq!(action.task_hypothesis_id.as_deref(), Some("hypothesis-b"));

        assert_eq!(
            scoped_task_feedback_contract(&IslandActionKind::ChooseTaskAlternative),
            Some(("corrected", "hypothesis", true))
        );
        assert_eq!(
            scoped_task_feedback_contract(&IslandActionKind::RejectSelectedTask),
            Some(("rejected", "task_summary", false))
        );
        assert_eq!(
            scoped_task_feedback_contract(&IslandActionKind::MarkSupportingWork),
            Some(("supporting_work", "relationship", true))
        );
        assert_eq!(
            scoped_task_feedback_contract(&IslandActionKind::MarkUnrelatedActivity),
            Some(("unrelated_activity", "relationship", true))
        );
        assert_eq!(
            scoped_task_feedback_contract(&IslandActionKind::MarkTaskCompleted),
            Some(("completed", "task_status", true))
        );
        assert_eq!(
            scoped_task_feedback_contract(&IslandActionKind::ReactivateTask),
            Some(("reactivated", "task_status", true))
        );
    }

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

        assert_eq!(
            snapshot.continue_decision_id.as_deref(),
            Some("decision-test")
        );
        assert_eq!(snapshot.continue_freshness.as_deref(), Some("current"));
        assert_eq!(
            snapshot.resume_headline.as_deref(),
            Some("Ready to continue")
        );
        assert_eq!(snapshot.resume_point.as_deref(), Some("PRODUCT.md"));
    }

    #[test]
    fn snapshot_from_status_marks_remembered_continue_stale_on_event_only_evidence() {
        let _guard = TEST_LOCK.lock().unwrap();
        remember_continue_for_test(1, 7, 12, "current");
        let status = status_for_island_freshness(1, 7, 13, 10_000);

        let snapshot = snapshot_from_status(&status, SessionIslandState::RecordingCompact);

        assert_eq!(
            snapshot.continue_decision_id.as_deref(),
            Some("decision-test")
        );
        assert_eq!(snapshot.continue_freshness.as_deref(), Some("new_evidence"));
        assert_eq!(snapshot.resume_headline.as_deref(), Some("New evidence"));
        assert_eq!(snapshot.resume_detail.as_deref(), Some("Refresh Continue"));
        assert_eq!(snapshot.resume_point.as_deref(), Some("PRODUCT.md"));
    }

    #[test]
    fn capture_status_session_change_preserves_atomic_continue_as_stale() {
        let _guard = TEST_LOCK.lock().unwrap();
        remember_continue_for_test(1, 7, 12, "current");
        if let Ok(mut slot) = LAST_CONTINUE_ISLAND_STATE.lock() {
            slot.as_mut().unwrap().session_id = Some("previous-session".to_string());
        }
        let status = status_for_island_freshness(1, 7, 12, 10_000);

        let snapshot = snapshot_from_status(&status, SessionIslandState::RecordingCompact);

        assert_eq!(
            snapshot.continue_decision_id.as_deref(),
            Some("decision-test")
        );
        assert_eq!(snapshot.continue_freshness.as_deref(), Some("new_evidence"));
        let state = snapshot.island_continue_state.unwrap();
        assert_eq!(state.display_state, IslandDisplayState::NeedsRefresh);
        assert_eq!(state.decision_id.as_deref(), Some("decision-test"));
        assert_eq!(
            state.available_actions.first().map(|action| &action.kind),
            Some(&IslandActionKind::RefreshContinue)
        );
    }

    #[test]
    fn processing_capture_status_does_not_erase_latest_continue_state() {
        let _guard = TEST_LOCK.lock().unwrap();
        remember_continue_for_test(1, 7, 12, "current");
        let status = status_for_island_freshness(1, 7, 12, 10_000);

        let snapshot = snapshot_from_status(&status, SessionIslandState::Processing);

        assert_eq!(
            snapshot.continue_decision_id.as_deref(),
            Some("decision-test")
        );
        assert_eq!(snapshot.continue_freshness.as_deref(), Some("current"));
        assert_eq!(
            snapshot
                .island_continue_state
                .as_ref()
                .map(|state| &state.display_state),
            Some(&IslandDisplayState::ContinueReady)
        );
    }

    #[test]
    fn swift_action_contract_keeps_capture_secondary_to_continue() {
        let source = include_str!("../macos/SessionIslandPanel.swift");

        assert!(source.contains(
            "case .localMemoryWarming:\n            priority = [.refreshContinue, .openSmalltalk, .captureEvidenceNow]"
        ));
        assert!(source.contains(
            "if displayState == .localMemoryWarming,\n           let updateEvidence = firstEnabledAction(in: [.captureEvidenceNow])"
        ));
        assert!(source.contains("return compact ? \"Update\" : \"Update local evidence\""));
        assert!(source.contains(
            "case .inspectOnly:\n            priority = [.inspectEvidence, .refreshContinue, .openSmalltalk]"
        ));
    }

    #[test]
    fn island_base_request_does_not_write_full_audit_output() {
        let request = island_continue_decision_request();

        assert_eq!(request.mode.as_deref(), Some("normal"));
        assert_eq!(request.rebuild_layers, Some(false));
        assert_eq!(request.micro_inference_enabled, Some(true));
        assert_eq!(request.activity_recap_model_enabled, Some(true));
        assert_eq!(request.max_candidates_for_model, Some(5));
        assert_eq!(request.audit_output_enabled, Some(false));
        assert!(matches!(
            request.audit_mode,
            Some(crate::continuation::ContinueAuditMode::None)
        ));
    }

    #[test]
    fn island_primary_routes_do_not_include_diagnostic_or_deprecated_paths() {
        for route in ISLAND_ROUTE_INVENTORY {
            if route.allowed_in_primary_ui {
                assert!(
                    !matches!(
                        route.kind,
                        IslandRouteKind::DiagnosticCloudResume
                            | IslandRouteKind::DiagnosticSessionTrail
                            | IslandRouteKind::DeprecatedLegacyOpen
                    ),
                    "{} must not be primary because it is {:?}",
                    route.route_name,
                    route.kind
                );
                assert_ne!(
                    route.disposition,
                    IslandActionDisposition::DiagnosticOnly,
                    "{} must not expose diagnostic-only routes in primary UI",
                    route.route_name
                );
                assert_ne!(
                    route.disposition,
                    IslandActionDisposition::DeprecatedBlocked,
                    "{} must not expose deprecated routes in primary UI",
                    route.route_name
                );
            }
        }
    }

    #[test]
    fn island_native_wire_actions_are_classified_in_inventory() {
        let actions = [
            ("continue", "native_action_continue"),
            ("start_capture", "native_action_start_capture"),
            ("stop_capture", "native_action_stop_capture"),
            ("capture_once", "native_action_capture_once"),
            ("reconstruct_trail", "native_action_reconstruct_trail"),
            ("show_trail", "native_action_show_trail"),
            ("open_resume_point", "native_action_open_resume_point"),
            (
                "perform_continue_action",
                "native_action_perform_continue_action",
            ),
            ("open_main_window", "native_action_open_main_window"),
            ("resume_me", "native_action_resume_me"),
            ("toggle_expanded", "native_action_toggle_expanded"),
            ("collapse", "native_action_collapse"),
        ];

        for (wire_action, route_name) in actions {
            let payload = format!(r#"{{"action":"{wire_action}"}}"#);
            let _action: SessionIslandAction = serde_json::from_str(&payload)
                .unwrap_or_else(|error| panic!("{wire_action} did not deserialize: {error}"));
            assert!(
                ISLAND_ROUTE_INVENTORY
                    .iter()
                    .any(|route| route.route_name == route_name),
                "{wire_action} must have an explicit P4.01 route classification"
            );
        }
    }

    #[test]
    fn island_primary_open_requires_continue_decision_id() {
        let primary_opens: Vec<_> = ISLAND_ROUTE_INVENTORY
            .iter()
            .filter(|route| route.kind == IslandRouteKind::PrimaryContinueOpen)
            .collect();

        assert!(
            !primary_opens.is_empty(),
            "P4 island inventory must identify the primary Continue open route"
        );
        for route in primary_opens {
            assert!(
                route.requires_continue_decision_id,
                "{} must require continue_decision_id",
                route.route_name
            );
            assert!(
                route.replacement.contains("continue_decision_id"),
                "{} must document the Continue-id replacement path",
                route.route_name
            );
        }
    }

    #[test]
    fn island_legacy_action_aliases_are_blocked_from_primary_ui() {
        for route_name in ["native_action_resume_me", "native_action_reconstruct_trail"] {
            let route = ISLAND_ROUTE_INVENTORY
                .iter()
                .find(|route| route.route_name == route_name)
                .unwrap_or_else(|| panic!("missing island route inventory item {route_name}"));

            assert_eq!(
                route.disposition,
                IslandActionDisposition::DeprecatedBlocked
            );
            assert!(
                !route.allowed_in_primary_ui,
                "{} must stay out of the primary island UI",
                route.route_name
            );
        }
    }

    #[test]
    fn island_legacy_resume_routes_are_diagnostic_only() {
        for route_name in [
            "apply_cloud_resume_to_snapshot",
            "remember_cloud_resume_output_path",
            "legacy_command_run_cloud_resume",
            "legacy_command_build_resume_query_bundle",
            "legacy_command_get_native_resume_card",
            "legacy_command_get_native_storyboard_dossier",
        ] {
            let route = ISLAND_ROUTE_INVENTORY
                .iter()
                .find(|route| route.route_name == route_name)
                .unwrap_or_else(|| panic!("missing island route inventory item {route_name}"));

            assert_eq!(route.disposition, IslandActionDisposition::DiagnosticOnly);
            assert!(
                !route.allowed_in_primary_ui,
                "{} must not feed island primary behavior",
                route.route_name
            );
        }
    }

    #[test]
    fn island_state_replacements_use_continue_first_language() {
        let banned_copy_terms = [
            "recording",
            "recorder",
            "session",
            "trail",
            "reconstruct",
            "resume query",
            "cloud resume",
            "native resume",
            "resume me",
        ];

        for state in ISLAND_STATE_INVENTORY {
            let replacement = state.replacement_copy.to_lowercase();
            for banned in banned_copy_terms {
                assert!(
                    !replacement.contains(banned),
                    "{} replacement copy {:?} contains legacy term {:?}",
                    state.state_name,
                    state.replacement_copy,
                    banned
                );
            }
            if state.state_name == "resume_ready" {
                assert!(
                    state.requires_continue_decision_id,
                    "resume_ready must be backed by continue_decision_id"
                );
            }
        }
    }

    #[test]
    fn product_visible_island_copy_excludes_legacy_primary_terms() {
        let swift_panel = include_str!("../macos/SessionIslandPanel.swift").to_lowercase();
        let rust_contract = include_str!("session_island/contract.rs").to_lowercase();
        let forbidden_primary_terms = [
            "resume me",
            "resume ready",
            "trail reconstructing",
            "reconstruct trail",
            "cloud resume",
            "native resume",
            "resume query",
            "session resume",
            "recording session",
        ];

        for term in forbidden_primary_terms {
            assert!(
                !swift_panel.contains(term),
                "Swift island panel must not expose legacy primary copy term {term:?}"
            );
            assert!(
                !rust_contract.contains(term),
                "Island Continue state contract must not expose legacy primary copy term {term:?}"
            );
        }
    }

    #[test]
    fn island_continue_audit_writes_required_no_bypass_fields() {
        let audit_dir =
            std::env::temp_dir().join(format!("smalltalk-island-audit-{}", now_millis()));
        let mut state = IslandContinueState::error(2_000, Some("p1_suppressed".to_string()));
        state.decision_id = Some("decision-audit-test".to_string());
        state.display_state = IslandDisplayState::TargetSuppressed;
        state.available_actions = vec![IslandAvailableAction::enabled(
            IslandActionKind::InspectEvidence,
            "Inspect evidence",
            None,
        )];

        write_island_continue_audit(
            Some(&audit_dir.to_string_lossy()),
            &state,
            "user_pressed_continue",
            "island_primary",
            true,
            false,
            Some("p1_suppressed"),
        );

        let payload_path = audit_dir
            .join("decision")
            .join("island_continue_audit.json");
        let payload = std::fs::read_to_string(&payload_path).unwrap_or_else(|error| {
            panic!("missing island audit {}: {error}", payload_path.display())
        });
        let json: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(
            json.pointer("/island/state_schema")
                .and_then(|value| value.as_str()),
            Some("smalltalk.island_continue_state.v1")
        );
        assert_eq!(
            json.pointer("/island/source")
                .and_then(|value| value.as_str()),
            Some("island_primary")
        );
        assert_eq!(
            json.pointer("/island/open_attempted")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            json.pointer("/island/open_allowed")
                .and_then(|value| value.as_bool()),
            Some(false)
        );
        assert_eq!(
            json.pointer("/island/open_blocked_reason")
                .and_then(|value| value.as_str()),
            Some("p1_suppressed")
        );
        assert!(!payload.contains("https://"));
        assert!(!payload.contains("/Users/"));
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
            let mut island_continue_state = IslandContinueState::no_evidence(
                IslandFreshness {
                    evidence_watermark_ms: Some(10_000),
                    newest_evidence_ms: Some(10_000),
                    decision_updated_at_ms: Some(11_000),
                    decision_stale: false,
                },
                IslandStateContext {
                    local_memory_running: true,
                    has_local_memory: true,
                },
            );
            island_continue_state.decision_id = Some("decision-test".to_string());
            island_continue_state.display_state = IslandDisplayState::ContinueReady;
            island_continue_state.available_actions = vec![IslandAvailableAction::enabled(
                IslandActionKind::OpenContinueTarget,
                "Open Continue target",
                Some("decision-test".to_string()),
            )];
            *slot = Some(RememberedContinueIslandState {
                session_id: None,
                decision_id: "decision-test".to_string(),
                request_trigger: "manual".to_string(),
                task_turn_id: Some("task-test".to_string()),
                task_turn_revision: Some(1),
                task_confidence: 0.9,
                wording_source: "model_assisted".to_string(),
                target_selection_source: "local_validated_target_policy".to_string(),
                resume_headline: Some("Ready to continue".to_string()),
                resume_detail: Some("Editing was in progress.".to_string()),
                resume_point: Some("PRODUCT.md".to_string()),
                resume_warning: None,
                continue_freshness: freshness.to_string(),
                evidence_updated_at_ms: Some(10_000),
                decision_updated_at_ms: Some(11_000),
                continue_openable: true,
                feedback_or_open_watermark_ms: None,
                frame_count,
                signal_count,
                event_count,
                island_continue_state,
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
