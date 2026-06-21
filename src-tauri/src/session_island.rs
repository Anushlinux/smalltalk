use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

use crate::capture::CaptureStatus;

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static EXPANDED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize)]
pub struct SessionIslandSnapshot {
    pub state: SessionIslandState,
    pub session_id: Option<String>,
    pub elapsed_ms: u64,
    pub frame_count: u64,
    pub current_app: Option<String>,
    pub current_window: Option<String>,
    pub current_surface_kind: Option<String>,
    pub last_trigger: Option<String>,
    pub last_error: Option<String>,
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
    Error,
}

#[derive(Debug, Deserialize)]
struct SessionIslandAction {
    action: SessionIslandActionKind,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SessionIslandActionKind {
    StartCapture,
    StopCapture,
    CaptureOnce,
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
            current_app: None,
            current_window: None,
            current_surface_kind: None,
            last_trigger: None,
            last_error: None,
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

    SessionIslandSnapshot {
        state,
        session_id,
        elapsed_ms,
        frame_count: status.frame_count.max(0) as u64,
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
        last_error: status.last_error.clone(),
        privacy_label,
        is_sensitive,
    }
}

fn clean_one_line(value: Option<&str>) -> Option<String> {
    value
        .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|value| !value.is_empty())
}

fn is_sensitive_privacy_label(label: &str) -> bool {
    !matches!(label, "normal" | "ok" | "allowed")
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
        SessionIslandActionKind::OpenMainWindow => open_main_window(),
        SessionIslandActionKind::ResumeMe => {
            open_main_window();
            if let Some(app) = APP_HANDLE.get() {
                let _ = app.emit("session-island-resume-requested", serde_json::json!({}));
            }
        }
        SessionIslandActionKind::ToggleExpanded => toggle_expanded_from_native(),
        SessionIslandActionKind::Collapse => set_session_island_expanded(false),
    }
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
