use super::process_runner::{CancellationToken, ProcessSpec};

#[cfg(debug_assertions)]
use std::ffi::OsString;
#[cfg(debug_assertions)]
use std::path::PathBuf;
#[cfg(debug_assertions)]
use std::thread;
#[cfg(debug_assertions)]
use std::time::{Duration, Instant};

#[cfg(debug_assertions)]
const PROCESS_FAULT_OPERATIONS: &[&str] = &[
    "screencapturekit_full_display",
    "screencapturekit_active_window",
    "screencapture_display_fallback",
    "screencapture_active_window_fallback",
    "accessibility_snapshot",
    "window_snapshot",
    "vision_ocr",
    "tesseract_ocr_fallback",
];

#[cfg(debug_assertions)]
const CANCELLATION_POINTS: &[&str] = &[
    "before_accessibility",
    "before_window_snapshot",
    "before_screenshot",
    "before_ocr",
    "before_persistence",
];

/// Development-only capture failure selector.
///
/// Set `SMALLTALK_CAPTURE_FAULT=<operation>:<mode>` before starting `tauri dev`.
/// Supported process modes are `hang`, `abnormal_exit`, `invalid_json`,
/// `oversized_stdout`, `oversized_stderr`, and `failure`. A release build does
/// not read this environment variable and compiles every hook to a no-op.
#[cfg(debug_assertions)]
fn requested_mode(point: &str) -> Option<String> {
    let raw = std::env::var("SMALLTALK_CAPTURE_FAULT").ok()?;
    let (requested_point, mode) = raw.split_once(':')?;
    (requested_point == point).then(|| mode.to_string())
}

pub(super) fn apply_process_fault(spec: ProcessSpec) -> ProcessSpec {
    #[cfg(debug_assertions)]
    {
        if !PROCESS_FAULT_OPERATIONS.contains(&spec.operation) {
            return spec;
        }
        let Some(mode) = requested_mode(spec.operation) else {
            return spec;
        };
        return inject_process_mode(spec, &mode);
    }

    #[cfg(not(debug_assertions))]
    spec
}

#[cfg(debug_assertions)]
fn inject_process_mode(spec: ProcessSpec, mode: &str) -> ProcessSpec {
    let script = match mode {
        "hang" => "sleep 30",
        "abnormal_exit" => "kill -ABRT $$",
        "invalid_json" => "printf '{invalid-json'",
        "oversized_stdout" => "yes stdout",
        "oversized_stderr" => "yes stderr >&2",
        "failure" => "exit 9",
        _ => return spec,
    };
    ProcessSpec {
        program: PathBuf::from("/bin/sh"),
        args: vec![OsString::from("-c"), OsString::from(script)],
        ..spec
    }
}

pub(super) fn cancellation_checkpoint(
    point: &str,
    cancellation: Option<&CancellationToken>,
) -> Result<(), String> {
    #[cfg(debug_assertions)]
    {
        if CANCELLATION_POINTS.contains(&point)
            && requested_mode(point).as_deref() == Some("wait_for_cancellation")
        {
            let started = Instant::now();
            while !cancellation.is_some_and(CancellationToken::is_cancelled)
                && started.elapsed() < Duration::from_secs(30)
            {
                thread::sleep(Duration::from_millis(5));
            }
            return if cancellation.is_some_and(CancellationToken::is_cancelled) {
                Err(format!("capture cancelled at injected stage {point}"))
            } else {
                Err(format!(
                    "injected stage {point} exceeded its safety deadline"
                ))
            };
        }
    }

    let _ = (point, cancellation);
    Ok(())
}

pub(super) fn panic_if_requested(point: &str) {
    #[cfg(debug_assertions)]
    if point == "capture_worker" && requested_mode(point).as_deref() == Some("panic") {
        panic!("development-only injected panic at {point}");
    }

    let _ = point;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fault_catalog_covers_every_external_failure_shape() {
        for mode in [
            "hang",
            "abnormal_exit",
            "invalid_json",
            "oversized_stdout",
            "oversized_stderr",
            "failure",
        ] {
            let original = ProcessSpec::new("fault_test", "/usr/bin/true", Duration::from_secs(1));
            let injected = inject_process_mode(original, mode);
            assert_eq!(injected.program, PathBuf::from("/bin/sh"));
            assert_eq!(injected.args.len(), 2);
        }
    }

    #[test]
    fn development_faults_cover_every_capture_helper_and_fallback() {
        assert_eq!(PROCESS_FAULT_OPERATIONS.len(), 9);
        for operation in PROCESS_FAULT_OPERATIONS {
            let injected = inject_process_mode(
                ProcessSpec::new(operation, "/usr/bin/true", Duration::from_secs(1)),
                "hang",
            );
            assert_eq!(injected.program, PathBuf::from("/bin/sh"));
        }
    }

    #[test]
    fn development_faults_cover_every_major_cancellation_stage_and_worker_panic() {
        assert_eq!(
            CANCELLATION_POINTS,
            [
                "before_accessibility",
                "before_window_snapshot",
                "before_screenshot",
                "before_ocr",
                "before_persistence",
            ]
        );
        assert_eq!("capture_worker", "capture_worker");
    }

    #[test]
    fn cancellation_checkpoint_is_a_no_op_without_an_armed_fault() {
        cancellation_checkpoint("before_screenshot", Some(&CancellationToken::default())).unwrap();
    }
}
