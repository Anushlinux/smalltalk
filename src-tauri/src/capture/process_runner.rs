use serde::Serialize;
use std::ffi::OsString;
use std::io::Read;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const POLL_INTERVAL: Duration = Duration::from_millis(5);
const MAX_ARGUMENT_COUNT: usize = 64;
const MAX_ARGUMENT_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, Default)]
pub(super) struct CancellationToken {
    cancelled: Arc<AtomicBool>,
    externally_cancelled: Option<Arc<AtomicBool>>,
}

impl CancellationToken {
    pub(super) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub(super) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
            || self
                .externally_cancelled
                .as_ref()
                .is_some_and(|cancelled| cancelled.load(Ordering::Acquire))
    }

    pub(super) fn linked_to(mut self, cancelled: Arc<AtomicBool>) -> Self {
        self.externally_cancelled = Some(cancelled);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ProcessExitCategory {
    Success,
    StructuredHelperError,
    InvalidResponse,
    NonZeroExit,
    SignalOrAbnormalTermination,
    Timeout,
    Cancelled,
    LaunchFailure,
    OutputLimitExceeded,
}

impl ProcessExitCategory {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::StructuredHelperError => "structured_helper_error",
            Self::InvalidResponse => "invalid_response",
            Self::NonZeroExit => "non_zero_exit",
            Self::SignalOrAbnormalTermination => "signal_or_abnormal_termination",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
            Self::LaunchFailure => "launch_failure",
            Self::OutputLimitExceeded => "output_limit_exceeded",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ProcessSpec {
    pub operation: &'static str,
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub deadline: Duration,
    pub stdout_limit: usize,
    pub stderr_limit: usize,
}

impl ProcessSpec {
    pub(super) fn new(
        operation: &'static str,
        program: impl Into<PathBuf>,
        deadline: Duration,
    ) -> Self {
        Self {
            operation,
            program: program.into(),
            args: Vec::new(),
            deadline,
            stdout_limit: 1024 * 1024,
            stderr_limit: 128 * 1024,
        }
    }

    pub(super) fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub(super) fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub(super) fn output_limits(mut self, stdout_limit: usize, stderr_limit: usize) -> Self {
        self.stdout_limit = stdout_limit;
        self.stderr_limit = stderr_limit;
        self
    }
}

#[derive(Debug)]
pub(super) struct ProcessOutput {
    pub category: ProcessExitCategory,
    #[allow(dead_code)]
    pub pid: Option<u32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub duration_ms: i64,
}

impl ProcessOutput {
    pub(super) fn success(&self) -> bool {
        self.category == ProcessExitCategory::Success
    }

    pub(super) fn stdout_text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).trim().to_string()
    }

    pub(super) fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).trim().to_string()
    }

    pub(super) fn reclassify(&mut self, category: ProcessExitCategory) {
        if self.category == category {
            return;
        }
        let diagnostics = diagnostics();
        if self.category == ProcessExitCategory::Success {
            let _ = diagnostics.helper_successes.fetch_update(
                Ordering::Relaxed,
                Ordering::Relaxed,
                |value| Some(value.saturating_sub(1)),
            );
        }
        self.category = category;
        record_category(diagnostics, category);
        if let Some(last) = diagnostics
            .last
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_mut()
        {
            last.category = Some(category);
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub(super) struct ProcessDiagnosticsSnapshot {
    pub helper_launches: u64,
    pub helper_successes: u64,
    pub helper_timeouts: u64,
    pub helper_timeouts_reaped: u64,
    pub helper_cancellations: u64,
    pub helper_abnormal_exits: u64,
    pub helper_output_limit_failures: u64,
    pub helper_launch_failures: u64,
    pub active_child_processes: u64,
    pub current_operation_class: Option<String>,
    pub current_operation_started_at_ms: Option<i64>,
    pub last_operation_class: Option<String>,
    pub last_operation_duration_ms: Option<i64>,
    pub last_safe_error_category: Option<String>,
}

#[derive(Debug, Clone)]
struct OperationRecord {
    name: String,
    started_at_ms: i64,
    duration_ms: Option<i64>,
    category: Option<ProcessExitCategory>,
}

#[derive(Debug, Default)]
struct ProcessDiagnostics {
    helper_launches: AtomicU64,
    helper_successes: AtomicU64,
    helper_timeouts: AtomicU64,
    helper_timeouts_reaped: AtomicU64,
    helper_cancellations: AtomicU64,
    helper_abnormal_exits: AtomicU64,
    helper_output_limit_failures: AtomicU64,
    helper_launch_failures: AtomicU64,
    active_child_processes: AtomicU64,
    current: Mutex<Option<OperationRecord>>,
    last: Mutex<Option<OperationRecord>>,
}

static PROCESS_DIAGNOSTICS: OnceLock<ProcessDiagnostics> = OnceLock::new();

fn diagnostics() -> &'static ProcessDiagnostics {
    PROCESS_DIAGNOSTICS.get_or_init(ProcessDiagnostics::default)
}

pub(super) fn diagnostics_snapshot() -> ProcessDiagnosticsSnapshot {
    let diagnostics = diagnostics();
    let current = diagnostics
        .current
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    let last = diagnostics
        .last
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    ProcessDiagnosticsSnapshot {
        helper_launches: diagnostics.helper_launches.load(Ordering::Relaxed),
        helper_successes: diagnostics.helper_successes.load(Ordering::Relaxed),
        helper_timeouts: diagnostics.helper_timeouts.load(Ordering::Relaxed),
        helper_timeouts_reaped: diagnostics.helper_timeouts_reaped.load(Ordering::Relaxed),
        helper_cancellations: diagnostics.helper_cancellations.load(Ordering::Relaxed),
        helper_abnormal_exits: diagnostics.helper_abnormal_exits.load(Ordering::Relaxed),
        helper_output_limit_failures: diagnostics
            .helper_output_limit_failures
            .load(Ordering::Relaxed),
        helper_launch_failures: diagnostics.helper_launch_failures.load(Ordering::Relaxed),
        active_child_processes: diagnostics.active_child_processes.load(Ordering::Relaxed),
        current_operation_class: current.as_ref().map(|record| record.name.clone()),
        current_operation_started_at_ms: current.as_ref().map(|record| record.started_at_ms),
        last_operation_class: last.as_ref().map(|record| record.name.clone()),
        last_operation_duration_ms: last.as_ref().and_then(|record| record.duration_ms),
        last_safe_error_category: last
            .as_ref()
            .and_then(|record| record.category)
            .filter(|category| *category != ProcessExitCategory::Success)
            .map(|category| category.as_str().to_string()),
    }
}

pub(super) fn run_process(spec: ProcessSpec, cancellation: &CancellationToken) -> ProcessOutput {
    let spec = super::fault_injection::apply_process_fault(spec);
    let started = Instant::now();
    let started_at_ms = now_millis();
    if cancellation.is_cancelled() {
        return completed_without_launch(
            &spec,
            started_at_ms,
            started,
            ProcessExitCategory::Cancelled,
            "operation cancelled before launch",
        );
    }
    if let Err(error) = validate_spec(&spec) {
        return completed_without_launch(
            &spec,
            started_at_ms,
            started,
            ProcessExitCategory::LaunchFailure,
            &error,
        );
    }

    let diagnostics = diagnostics();
    diagnostics.helper_launches.fetch_add(1, Ordering::Relaxed);
    *diagnostics
        .current
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(OperationRecord {
        name: spec.operation.to_string(),
        started_at_ms,
        duration_ms: None,
        category: None,
    });

    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Each helper is its own process group. Killing the group also terminates
    // nested tools such as the osascript process used by the Accessibility
    // helper, so Stop cannot leave an orphan behind.
    configure_process_group(&mut command);

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            diagnostics
                .helper_launch_failures
                .fetch_add(1, Ordering::Relaxed);
            return finish(
                &spec,
                started_at_ms,
                started,
                ProcessExitCategory::LaunchFailure,
                None,
                None,
                Vec::new(),
                error.to_string().into_bytes(),
            );
        }
    };
    diagnostics
        .active_child_processes
        .fetch_add(1, Ordering::Relaxed);
    let pid = child.id();

    let output_exceeded = Arc::new(AtomicBool::new(false));
    let stdout_reader = spawn_bounded_reader(
        child.stdout.take(),
        spec.stdout_limit,
        output_exceeded.clone(),
    );
    let stderr_reader = spawn_bounded_reader(
        child.stderr.take(),
        spec.stderr_limit,
        output_exceeded.clone(),
    );

    let mut forced_category = None;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if cancellation.is_cancelled() => {
                forced_category = Some(ProcessExitCategory::Cancelled);
                terminate_process_group(&mut child);
                break child.wait().ok();
            }
            Ok(None) if output_exceeded.load(Ordering::Acquire) => {
                forced_category = Some(ProcessExitCategory::OutputLimitExceeded);
                terminate_process_group(&mut child);
                break child.wait().ok();
            }
            Ok(None) if started.elapsed() >= spec.deadline => {
                forced_category = Some(ProcessExitCategory::Timeout);
                terminate_process_group(&mut child);
                break child.wait().ok();
            }
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(_) => {
                forced_category = Some(ProcessExitCategory::SignalOrAbnormalTermination);
                terminate_process_group(&mut child);
                break child.wait().ok();
            }
        }
    };

    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();
    diagnostics
        .active_child_processes
        .fetch_sub(1, Ordering::Relaxed);
    let category = forced_category.unwrap_or_else(|| {
        if output_exceeded.load(Ordering::Acquire) {
            ProcessExitCategory::OutputLimitExceeded
        } else {
            classify_status(status.as_ref())
        }
    });
    record_category(diagnostics, category);
    if category == ProcessExitCategory::Timeout {
        // Reaching this point means the deadline path killed the process
        // group, waited for the child, and joined both bounded output readers.
        diagnostics
            .helper_timeouts_reaped
            .fetch_add(1, Ordering::Relaxed);
    }
    finish(
        &spec,
        started_at_ms,
        started,
        category,
        status,
        Some(pid),
        stdout,
        stderr,
    )
}

/// Long-lived capture sidecars use the same process-group ownership rule as
/// one-shot helpers. This keeps a sidecar and any descendants inside one
/// lifecycle boundary owned by the capture worker.
pub(super) fn configure_process_group(command: &mut Command) {
    #[cfg(unix)]
    {
        command.process_group(0);
    }
}

/// Terminates the full helper process group and unconditionally reaps the
/// direct child. It is intentionally best-effort because shutdown must keep
/// progressing even when the child has already exited.
pub(super) fn terminate_and_reap(child: &mut Child) {
    terminate_process_group(child);
    let _ = child.wait();
}

pub(super) fn record_managed_child_started() {
    let diagnostics = diagnostics();
    diagnostics.helper_launches.fetch_add(1, Ordering::Relaxed);
    diagnostics
        .active_child_processes
        .fetch_add(1, Ordering::Relaxed);
}

pub(super) fn record_managed_child_stopped() {
    diagnostics()
        .active_child_processes
        .fetch_sub(1, Ordering::Relaxed);
}

fn completed_without_launch(
    spec: &ProcessSpec,
    started_at_ms: i64,
    started: Instant,
    category: ProcessExitCategory,
    message: &str,
) -> ProcessOutput {
    let diagnostics = diagnostics();
    record_category(diagnostics, category);
    finish(
        spec,
        started_at_ms,
        started,
        category,
        None,
        None,
        Vec::new(),
        message.as_bytes().to_vec(),
    )
}

fn finish(
    spec: &ProcessSpec,
    started_at_ms: i64,
    started: Instant,
    category: ProcessExitCategory,
    _status: Option<ExitStatus>,
    pid: Option<u32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
) -> ProcessOutput {
    let duration_ms = started.elapsed().as_millis().min(i64::MAX as u128) as i64;
    let record = OperationRecord {
        name: spec.operation.to_string(),
        started_at_ms,
        duration_ms: Some(duration_ms),
        category: Some(category),
    };
    let diagnostics = diagnostics();
    *diagnostics
        .last
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(record);
    *diagnostics
        .current
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = None;
    ProcessOutput {
        category,
        pid,
        stdout,
        stderr,
        duration_ms,
    }
}

fn record_category(diagnostics: &ProcessDiagnostics, category: ProcessExitCategory) {
    match category {
        ProcessExitCategory::Success => {
            diagnostics.helper_successes.fetch_add(1, Ordering::Relaxed);
        }
        ProcessExitCategory::Timeout => {
            diagnostics.helper_timeouts.fetch_add(1, Ordering::Relaxed);
        }
        ProcessExitCategory::Cancelled => {
            diagnostics
                .helper_cancellations
                .fetch_add(1, Ordering::Relaxed);
        }
        ProcessExitCategory::SignalOrAbnormalTermination => {
            diagnostics
                .helper_abnormal_exits
                .fetch_add(1, Ordering::Relaxed);
        }
        ProcessExitCategory::OutputLimitExceeded => {
            diagnostics
                .helper_output_limit_failures
                .fetch_add(1, Ordering::Relaxed);
        }
        ProcessExitCategory::LaunchFailure => {
            diagnostics
                .helper_launch_failures
                .fetch_add(1, Ordering::Relaxed);
        }
        ProcessExitCategory::StructuredHelperError
        | ProcessExitCategory::InvalidResponse
        | ProcessExitCategory::NonZeroExit => {}
    }
}

fn validate_spec(spec: &ProcessSpec) -> Result<(), String> {
    if !spec.program.is_absolute() {
        return Err("helper executable path must be absolute".to_string());
    }
    if !spec.program.is_file() {
        return Err(format!(
            "helper executable is unavailable: {}",
            spec.program.display()
        ));
    }
    if spec.deadline.is_zero() {
        return Err("helper deadline must be greater than zero".to_string());
    }
    if spec.args.len() > MAX_ARGUMENT_COUNT {
        return Err(format!(
            "helper argument count exceeds {MAX_ARGUMENT_COUNT}"
        ));
    }
    let argument_bytes = spec.args.iter().map(os_string_bytes).sum::<usize>();
    if argument_bytes > MAX_ARGUMENT_BYTES {
        return Err(format!(
            "helper arguments exceed {MAX_ARGUMENT_BYTES} bytes"
        ));
    }
    if spec.stdout_limit == 0 || spec.stderr_limit == 0 {
        return Err("helper output limits must be greater than zero".to_string());
    }
    Ok(())
}

#[cfg(unix)]
fn os_string_bytes(value: &OsString) -> usize {
    value.as_os_str().as_bytes().len()
}

#[cfg(not(unix))]
fn os_string_bytes(value: &OsString) -> usize {
    value.to_string_lossy().len()
}

fn spawn_bounded_reader<R: Read + Send + 'static>(
    reader: Option<R>,
    limit: usize,
    exceeded: Arc<AtomicBool>,
) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let Some(mut reader) = reader else {
            return Vec::new();
        };
        let mut stored = Vec::with_capacity(limit.min(16 * 1024));
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    let remaining = limit.saturating_sub(stored.len());
                    let keep = remaining.min(read);
                    stored.extend_from_slice(&buffer[..keep]);
                    if keep < read {
                        exceeded.store(true, Ordering::Release);
                    }
                }
                Err(_) => break,
            }
        }
        stored
    })
}

fn classify_status(status: Option<&ExitStatus>) -> ProcessExitCategory {
    let Some(status) = status else {
        return ProcessExitCategory::SignalOrAbnormalTermination;
    };
    #[cfg(unix)]
    if status.signal().is_some() {
        return ProcessExitCategory::SignalOrAbnormalTermination;
    }
    if status.success() {
        ProcessExitCategory::Success
    } else {
        ProcessExitCategory::NonZeroExit
    }
}

fn terminate_process_group(child: &mut std::process::Child) {
    #[cfg(unix)]
    unsafe {
        let process_group = -(child.id() as i32);
        let _ = libc::kill(process_group, libc::SIGKILL);
    }
    let _ = child.kill();
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

pub(super) fn find_absolute_command(name: &str) -> Option<PathBuf> {
    if name.contains('/') {
        let path = PathBuf::from(name);
        return (path.is_absolute() && path.is_file()).then_some(path);
    }
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|directory| directory.join(name))
            .find(|candidate| candidate.is_file())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn shell(script: &str, deadline: Duration) -> ProcessSpec {
        ProcessSpec::new("test_helper", "/bin/sh", deadline)
            .args([OsString::from("-c"), OsString::from(script)])
            .output_limits(1024, 1024)
    }

    #[test]
    fn bounded_runner_classifies_success_and_non_zero_exit() {
        let token = CancellationToken::default();
        let success = run_process(shell("printf ready", Duration::from_secs(1)), &token);
        assert_eq!(success.category, ProcessExitCategory::Success);
        assert_eq!(success.stdout_text(), "ready");

        let failed = run_process(
            shell("printf nope >&2; exit 7", Duration::from_secs(1)),
            &token,
        );
        assert_eq!(failed.category, ProcessExitCategory::NonZeroExit);
        assert_eq!(failed.stderr_text(), "nope");
    }

    #[test]
    fn bounded_runner_timeout_kills_and_reaps_child() {
        let token = CancellationToken::default();
        let result = run_process(shell("sleep 10", Duration::from_millis(40)), &token);
        assert_eq!(result.category, ProcessExitCategory::Timeout);
        assert_process_reaped(result.pid);
    }

    #[test]
    fn bounded_runner_cancellation_kills_and_reaps_child() {
        let token = CancellationToken::default();
        let thread_token = token.clone();
        let (tx, rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            tx.send(()).unwrap();
            run_process(shell("sleep 10", Duration::from_secs(5)), &thread_token)
        });
        rx.recv().unwrap();
        thread::sleep(Duration::from_millis(20));
        token.cancel();
        let result = worker.join().unwrap();
        assert_eq!(result.category, ProcessExitCategory::Cancelled);
        assert_process_reaped(result.pid);
    }

    #[test]
    fn linked_workload_cancellation_kills_and_reaps_child() {
        let workload_cancelled = Arc::new(AtomicBool::new(false));
        let token = CancellationToken::default().linked_to(Arc::clone(&workload_cancelled));
        let worker =
            thread::spawn(move || run_process(shell("sleep 10", Duration::from_secs(5)), &token));
        thread::sleep(Duration::from_millis(20));
        workload_cancelled.store(true, Ordering::Release);
        let result = worker.join().unwrap();
        assert_eq!(result.category, ProcessExitCategory::Cancelled);
        assert_process_reaped(result.pid);
    }

    #[test]
    fn stop_during_helper_execution_finishes_inside_two_second_gate() {
        let token = CancellationToken::default();
        let thread_token = token.clone();
        let started = Instant::now();
        let worker = thread::spawn(move || {
            run_process(shell("sleep 30", Duration::from_secs(20)), &thread_token)
        });
        thread::sleep(Duration::from_millis(20));
        token.cancel();
        let result = worker.join().unwrap();

        assert_eq!(result.category, ProcessExitCategory::Cancelled);
        assert!(started.elapsed() < Duration::from_secs(2));
        assert_process_reaped(result.pid);
    }

    #[test]
    fn failed_helper_is_followed_by_a_successful_helper() {
        let token = CancellationToken::default();
        let failed = run_process(shell("exit 9", Duration::from_secs(1)), &token);
        assert_eq!(failed.category, ProcessExitCategory::NonZeroExit);

        let recovered = run_process(shell("printf recovered", Duration::from_secs(1)), &token);
        assert_eq!(recovered.category, ProcessExitCategory::Success);
        assert_eq!(recovered.stdout_text(), "recovered");
    }

    #[test]
    fn bounded_runner_classifies_abnormal_exit() {
        let result = run_process(
            shell("kill -ABRT $$", Duration::from_secs(1)),
            &CancellationToken::default(),
        );
        assert_eq!(
            result.category,
            ProcessExitCategory::SignalOrAbnormalTermination
        );
    }

    #[test]
    fn bounded_runner_enforces_stdout_and_stderr_limits() {
        for script in ["yes x | head -c 4096", "yes x | head -c 4096 >&2"] {
            let result = run_process(
                shell(script, Duration::from_secs(1)).output_limits(64, 64),
                &CancellationToken::default(),
            );
            assert_eq!(result.category, ProcessExitCategory::OutputLimitExceeded);
            assert_process_reaped(result.pid);
        }
    }

    #[test]
    fn bounded_runner_requires_an_absolute_executable() {
        let result = run_process(
            ProcessSpec::new("invalid", "sh", Duration::from_secs(1)),
            &CancellationToken::default(),
        );
        assert_eq!(result.category, ProcessExitCategory::LaunchFailure);
    }

    #[cfg(unix)]
    fn assert_process_reaped(pid: Option<u32>) {
        let pid = pid.expect("launched process id");
        let result = unsafe { libc::kill(pid as i32, 0) };
        assert_eq!(
            result, -1,
            "process {pid} still exists after runner returned"
        );
    }

    #[cfg(not(unix))]
    fn assert_process_reaped(_pid: Option<u32>) {}
}
