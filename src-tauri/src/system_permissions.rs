use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PermissionKind {
    ScreenRecording,
    Accessibility,
    InputMonitoring,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct RequiredPermissionStatus {
    pub key: String,
    pub granted: bool,
    pub can_request: bool,
    pub restart_required: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct AppPermissionsStatus {
    pub permissions: Vec<RequiredPermissionStatus>,
    pub all_granted: bool,
}

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: core_foundation::dictionary::CFDictionaryRef) -> u8;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
}

#[cfg(target_os = "macos")]
fn accessibility_is_granted() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

#[cfg(not(target_os = "macos"))]
fn accessibility_is_granted() -> bool {
    false
}

#[cfg(target_os = "macos")]
fn input_monitoring_is_granted() -> bool {
    unsafe { CGPreflightListenEventAccess() }
}

#[cfg(not(target_os = "macos"))]
fn input_monitoring_is_granted() -> bool {
    false
}

fn permission_status(
    key: &str,
    granted: bool,
    can_request: bool,
    restart_required: bool,
) -> RequiredPermissionStatus {
    RequiredPermissionStatus {
        key: key.into(),
        granted,
        can_request,
        restart_required,
    }
}

fn app_permissions_status(app: &AppHandle) -> Result<AppPermissionsStatus, String> {
    let screen = crate::capture::screen_capture_permission_status(app)?;
    let supported = cfg!(target_os = "macos");
    let permissions = vec![
        permission_status(
            "screen_recording",
            screen.granted,
            screen.can_request,
            screen.restart_required,
        ),
        permission_status(
            "accessibility",
            accessibility_is_granted(),
            supported,
            false,
        ),
        permission_status(
            "input_monitoring",
            input_monitoring_is_granted(),
            supported,
            false,
        ),
    ];
    let all_granted = permissions.iter().all(|permission| permission.granted);
    Ok(AppPermissionsStatus {
        permissions,
        all_granted,
    })
}

#[tauri::command]
pub(crate) fn get_app_permissions_status(app: AppHandle) -> Result<AppPermissionsStatus, String> {
    app_permissions_status(&app)
}

#[cfg(target_os = "macos")]
fn request_accessibility_permission() {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;

    let options: CFDictionary<CFString, CFBoolean> = CFDictionary::from_CFType_pairs(&[(
        CFString::from_static_string("AXTrustedCheckOptionPrompt"),
        CFBoolean::true_value(),
    )]);
    unsafe {
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef());
    }
}

#[cfg(not(target_os = "macos"))]
fn request_accessibility_permission() {}

#[cfg(target_os = "macos")]
fn request_input_monitoring_permission() {
    unsafe {
        CGRequestListenEventAccess();
    }
}

#[cfg(not(target_os = "macos"))]
fn request_input_monitoring_permission() {}

#[tauri::command]
pub(crate) fn request_app_permission(
    app: AppHandle,
    permission: PermissionKind,
) -> Result<AppPermissionsStatus, String> {
    match permission {
        PermissionKind::ScreenRecording => {
            crate::capture::request_screen_capture_permission(app.clone())?;
        }
        PermissionKind::Accessibility => request_accessibility_permission(),
        PermissionKind::InputMonitoring => request_input_monitoring_permission(),
    }
    app_permissions_status(&app)
}

#[cfg(target_os = "macos")]
fn settings_url(permission: PermissionKind) -> &'static str {
    match permission {
        PermissionKind::ScreenRecording => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
        }
        PermissionKind::Accessibility => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
        }
        PermissionKind::InputMonitoring => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent"
        }
    }
}

#[tauri::command]
pub(crate) fn open_app_permission_settings(permission: PermissionKind) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(settings_url(permission))
            .spawn()
            .map_err(|error| format!("System Settings could not be opened: {error}"))?;
        return Ok(());
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = permission;
        Err("Permission settings are only available in the macOS desktop app.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::permission_status;
    use std::fs;
    use std::path::Path;

    #[test]
    fn permission_status_keeps_restart_state_separate_from_grant_state() {
        let status = permission_status("screen_recording", false, false, true);
        assert!(!status.granted);
        assert!(!status.can_request);
        assert!(status.restart_required);
    }

    #[test]
    fn production_privacy_manifest_declares_only_screen_capture() {
        let manifest = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Info.plist"))
            .expect("Info.plist must be readable");
        assert!(manifest.contains("NSScreenCaptureUsageDescription"));
        for forbidden in [
            "NSAppleEventsUsageDescription",
            "NSDocumentsFolderUsageDescription",
            "NSDownloadsFolderUsageDescription",
            "NSDesktopFolderUsageDescription",
            "NSMicrophoneUsageDescription",
            "NSAudioCaptureUsageDescription",
        ] {
            assert!(
                !manifest.contains(forbidden),
                "unexpected permission: {forbidden}"
            );
        }

        let helper = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/accessibility_snapshot.swift"),
        )
        .expect("accessibility helper source must be readable");
        assert!(!helper.contains("/usr/bin/osascript"));
        assert!(!helper.contains("tell application"));
        assert!(!helper.contains("System Events"));
    }
}
