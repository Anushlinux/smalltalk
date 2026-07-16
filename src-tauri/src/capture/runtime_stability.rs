//! Developer-only always-on measurement harness.
//!
//! Enable it on the normal app path with `SMALLTALK_SOAK_SCENARIO` and a stable
//! `SMALLTALK_SOAK_RUN_ID`. Reports contain counts and resource measurements
//! only. Captured text, titles, URLs, paths, pixels, and clipboard values are
//! deliberately absent.

use super::*;
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;

const POLICY_JSON: &str = include_str!("../../../docs/runtime-stability-policy-v1.json");

pub(super) fn auto_start_capture_requested() -> bool {
    auto_start_capture_value(
        std::env::var("SMALLTALK_SOAK_SCENARIO").is_ok(),
        std::env::var("SMALLTALK_SOAK_AUTO_START_CAPTURE")
            .ok()
            .as_deref(),
    )
}

pub(super) fn auto_stop_capture_requested() -> bool {
    harness_boolean_value(
        std::env::var("SMALLTALK_SOAK_SCENARIO").is_ok(),
        std::env::var("SMALLTALK_SOAK_AUTO_STOP_CAPTURE")
            .ok()
            .as_deref(),
    )
}

fn auto_start_capture_value(scenario_present: bool, value: Option<&str>) -> bool {
    harness_boolean_value(scenario_present, value)
}

fn harness_boolean_value(scenario_present: bool, value: Option<&str>) -> bool {
    scenario_present
        && value.is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
}

#[derive(Debug, Clone, Default, Serialize)]
struct SoakSample {
    schema: &'static str,
    run_id: String,
    scenario: String,
    sampled_at_ms: i64,
    elapsed_ms: u64,
    main_process_alive: bool,
    process_start_count: u64,
    cpu_percent: Option<f64>,
    resident_memory_bytes: Option<u64>,
    thread_count: Option<u64>,
    file_descriptor_count: Option<u64>,
    capture_runtime_state: String,
    session_state: Option<String>,
    helper_launches: u64,
    helper_abnormal_exits: u64,
    helper_timeouts: u64,
    helper_timeouts_reaped: u64,
    unhandled_helper_timeouts: u64,
    helper_cancellations: u64,
    active_child_processes: u64,
    current_helper_operation: Option<String>,
    last_helper_operation: Option<String>,
    last_helper_duration_ms: Option<i64>,
    last_helper_error_category: Option<String>,
    event_queue_depth: u64,
    event_queue_capacity: u64,
    event_queue_high_water_mark: u64,
    event_coalesced: u64,
    event_dropped: u64,
    workload_active_count: u64,
    workload_queue_depth: u64,
    workload_queue_capacity: u64,
    workload_queue_high_water_mark: u64,
    workload_cancelled_or_superseded: u64,
    audit_active: bool,
    audit_queued: u64,
    audit_completed: u64,
    audit_failed: u64,
    database_busy_time_ms: u64,
    database_busy_retry_count: u64,
    schema_initialization_count: u64,
    database_generation: u64,
    database_bytes: u64,
    wal_bytes: u64,
    snapshot_bytes: u64,
    audit_bytes: u64,
    row_counts: HashMap<String, i64>,
    capture_stores: i64,
    capture_skips: i64,
    ocr_attempts: i64,
    continue_requests: i64,
    continue_cache_hits: i64,
    continue_decision_row_growth: i64,
    status_p50_latency_us: u64,
    status_p95_latency_us: u64,
    status_response_bytes: u64,
    stop_latency_ms: Option<i64>,
    main_crash_reports: u64,
    helper_crash_reports: u64,
}

#[derive(Debug, Clone, Default, Serialize)]
struct SoakMetrics {
    schema: &'static str,
    warmup_seconds: u64,
    post_warmup_sample_count: u64,
    cpu_p95_percent: Option<f64>,
    cpu_p95_gate_percent: f64,
    cpu_p95_passed: Option<bool>,
    memory_growth_bytes_per_hour: Option<f64>,
    memory_growth_gate_bytes_per_hour: f64,
    memory_growth_passed: Option<bool>,
    maximum_event_queue_depth: u64,
    maximum_event_queue_capacity: u64,
    maximum_event_queue_high_water_mark: u64,
    maximum_workload_queue_depth: u64,
    maximum_workload_queue_capacity: u64,
    maximum_workload_queue_high_water_mark: u64,
    maximum_status_p95_latency_us: u64,
    maximum_status_response_bytes: u64,
}

#[derive(Debug, Clone, Default)]
struct CounterBaseline {
    helper_launches: u64,
    helper_abnormal_exits: u64,
    helper_timeouts: u64,
    helper_timeouts_reaped: u64,
    helper_cancellations: u64,
    event_coalesced: u64,
    event_dropped: u64,
    workload_cancelled_or_superseded: u64,
    audit_completed: u64,
    audit_failed: u64,
    database_busy_time_ms: u64,
    database_busy_retry_count: u64,
    schema_initialization_count: u64,
    capture_stores: i64,
    capture_skips: i64,
    ocr_attempts: i64,
    continue_requests: i64,
    continue_cache_hits: i64,
    continue_decision_rows: i64,
}

impl CounterBaseline {
    fn from_sample(sample: &SoakSample) -> Self {
        Self {
            helper_launches: sample.helper_launches,
            helper_abnormal_exits: sample.helper_abnormal_exits,
            helper_timeouts: sample.helper_timeouts,
            helper_timeouts_reaped: sample.helper_timeouts_reaped,
            helper_cancellations: sample.helper_cancellations,
            event_coalesced: sample.event_coalesced,
            event_dropped: sample.event_dropped,
            workload_cancelled_or_superseded: sample.workload_cancelled_or_superseded,
            audit_completed: sample.audit_completed,
            audit_failed: sample.audit_failed,
            database_busy_time_ms: sample.database_busy_time_ms,
            database_busy_retry_count: sample.database_busy_retry_count,
            schema_initialization_count: sample.schema_initialization_count,
            capture_stores: sample.capture_stores,
            capture_skips: sample.capture_skips,
            ocr_attempts: sample.ocr_attempts,
            continue_requests: sample.continue_requests,
            continue_cache_hits: sample.continue_cache_hits,
            continue_decision_rows: sample
                .row_counts
                .get("continue_decisions")
                .copied()
                .unwrap_or(0),
        }
    }

    fn apply(&self, sample: &mut SoakSample) {
        sample.helper_launches = sample.helper_launches.saturating_sub(self.helper_launches);
        sample.helper_abnormal_exits = sample
            .helper_abnormal_exits
            .saturating_sub(self.helper_abnormal_exits);
        sample.helper_timeouts = sample.helper_timeouts.saturating_sub(self.helper_timeouts);
        sample.helper_timeouts_reaped = sample
            .helper_timeouts_reaped
            .saturating_sub(self.helper_timeouts_reaped);
        sample.unhandled_helper_timeouts = sample
            .helper_timeouts
            .saturating_sub(sample.helper_timeouts_reaped);
        sample.helper_cancellations = sample
            .helper_cancellations
            .saturating_sub(self.helper_cancellations);
        sample.event_coalesced = sample.event_coalesced.saturating_sub(self.event_coalesced);
        sample.event_dropped = sample.event_dropped.saturating_sub(self.event_dropped);
        sample.workload_cancelled_or_superseded = sample
            .workload_cancelled_or_superseded
            .saturating_sub(self.workload_cancelled_or_superseded);
        sample.audit_completed = sample.audit_completed.saturating_sub(self.audit_completed);
        sample.audit_failed = sample.audit_failed.saturating_sub(self.audit_failed);
        sample.database_busy_time_ms = sample
            .database_busy_time_ms
            .saturating_sub(self.database_busy_time_ms);
        sample.database_busy_retry_count = sample
            .database_busy_retry_count
            .saturating_sub(self.database_busy_retry_count);
        sample.schema_initialization_count = sample
            .schema_initialization_count
            .saturating_sub(self.schema_initialization_count);
        sample.capture_stores = sample.capture_stores.saturating_sub(self.capture_stores);
        sample.capture_skips = sample.capture_skips.saturating_sub(self.capture_skips);
        sample.ocr_attempts = sample.ocr_attempts.saturating_sub(self.ocr_attempts);
        sample.continue_requests = sample
            .continue_requests
            .saturating_sub(self.continue_requests);
        sample.continue_cache_hits = sample
            .continue_cache_hits
            .saturating_sub(self.continue_cache_hits);
        sample.continue_decision_row_growth = sample
            .row_counts
            .get("continue_decisions")
            .copied()
            .unwrap_or(0)
            .saturating_sub(self.continue_decision_rows);
    }
}

#[cfg(debug_assertions)]
pub(super) fn start_if_requested(app: AppHandle) {
    let Ok(scenario) = std::env::var("SMALLTALK_SOAK_SCENARIO") else {
        return;
    };
    let run_id = std::env::var("SMALLTALK_SOAK_RUN_ID")
        .unwrap_or_else(|_| format!("{}-{}", scenario, now_millis()));
    let duration = std::env::var("SMALLTALK_SOAK_DURATION_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .or_else(|| {
            std::env::var("SMALLTALK_SOAK_DURATION_MINUTES")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .map(|minutes| Duration::from_secs(minutes.saturating_mul(60)))
        })
        .unwrap_or_else(|| Duration::from_secs(60 * 60));
    let interval = std::env::var("SMALLTALK_SOAK_SAMPLE_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| Duration::from_secs(seconds.clamp(1, 60)))
        .unwrap_or_else(|| Duration::from_secs(5));
    let _ = thread::Builder::new()
        .name("smalltalk-runtime-soak".to_string())
        .spawn(move || run_harness(app, run_id, scenario, duration, interval));
}

#[cfg(not(debug_assertions))]
pub(super) fn start_if_requested(_app: AppHandle) {}

#[cfg(debug_assertions)]
fn run_harness(
    app: AppHandle,
    run_id: String,
    scenario: String,
    duration: Duration,
    interval: Duration,
) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
        .join("output/runtime-stability")
        .join(&run_id);
    if fs::create_dir_all(&root).is_err() {
        return;
    }
    let starts_path = root.join("process-start-count.txt");
    let process_start_count = fs::read_to_string(&starts_path)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(0)
        .saturating_add(1);
    if fs::write(&starts_path, process_start_count.to_string()).is_err() {
        return;
    }
    let report_path = root.join("samples.jsonl");
    let policy_path = root.join("policy.json");
    let _ = fs::write(&policy_path, POLICY_JSON);
    let crash_baseline = crash_report_counts();
    let started = Instant::now();
    let counter_baseline = collect_sample(
        &app,
        &run_id,
        &scenario,
        Duration::ZERO,
        process_start_count,
    )
    .as_ref()
    .map(CounterBaseline::from_sample)
    .unwrap_or_default();
    let mut sample_count = 0_u64;
    let mut max_rss = 0_u64;
    let mut max_threads = 0_u64;
    let mut max_fds = 0_u64;
    let mut final_sample = None;
    let mut observed_samples = Vec::new();
    let auto_stop = auto_stop_capture_requested();

    while started.elapsed() <= duration {
        if let Some(mut sample) = collect_sample(
            &app,
            &run_id,
            &scenario,
            started.elapsed(),
            process_start_count,
        ) {
            counter_baseline.apply(&mut sample);
            max_rss = max_rss.max(sample.resident_memory_bytes.unwrap_or(0));
            max_threads = max_threads.max(sample.thread_count.unwrap_or(0));
            max_fds = max_fds.max(sample.file_descriptor_count.unwrap_or(0));
            if append_json_line(&report_path, &sample).is_err() {
                break;
            }
            sample_count = sample_count.saturating_add(1);
            observed_samples.push(sample.clone());
            final_sample = Some(sample);
        }
        let sleep_started = Instant::now();
        while sleep_started.elapsed() < interval {
            thread::sleep(Duration::from_millis(200));
        }
    }

    let shutdown_error = if auto_stop {
        shutdown_capture(&app).err()
    } else {
        None
    };
    if auto_stop {
        if let Some(mut sample) = collect_sample(
            &app,
            &run_id,
            &scenario,
            started.elapsed(),
            process_start_count,
        ) {
            counter_baseline.apply(&mut sample);
            max_rss = max_rss.max(sample.resident_memory_bytes.unwrap_or(0));
            max_threads = max_threads.max(sample.thread_count.unwrap_or(0));
            max_fds = max_fds.max(sample.file_descriptor_count.unwrap_or(0));
            if append_json_line(&report_path, &sample).is_ok() {
                sample_count = sample_count.saturating_add(1);
                observed_samples.push(sample.clone());
                final_sample = Some(sample);
            }
        }
    }

    let metrics = build_metrics(&observed_samples, duration);
    if let Ok(encoded) = serde_json::to_vec_pretty(&metrics) {
        let _ = fs::write(root.join("metrics.json"), encoded);
    }

    let final_crashes = crash_report_counts();
    let quick_check = capture_paths(&app)
        .ok()
        .and_then(|paths| open_db_at_path(&paths.db_path, false).ok())
        .and_then(|conn| {
            conn.query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
                .ok()
        })
        .unwrap_or_else(|| "unavailable".to_string());
    let summary = format!(
        "# Smalltalk runtime soak\n\n- Run id: `{run_id}`\n- Scenario: `{scenario}`\n- Requested duration: {} seconds\n- Measured duration: {} seconds\n- Samples: {sample_count}\n- Process starts: {process_start_count}\n- Main crash delta: {}\n- Helper crash delta: {}\n- Peak resident memory: {max_rss} bytes\n- Peak threads: {max_threads}\n- Peak file descriptors: {max_fds}\n- Warm-up excluded from resource trend: {} seconds\n- Post-warm-up samples: {}\n- Post-warm-up CPU p95: {} percent (gate: {} percent, passed: {})\n- Post-warm-up memory growth: {} bytes/hour (gate: {} bytes/hour, passed: {})\n- Maximum event queue depth/capacity/high-water mark: {}/{}/{}\n- Maximum workload queue depth/capacity/high-water mark: {}/{}/{}\n- Maximum status p95: {} microseconds\n- Maximum status bytes: {}\n- SQLite quick check: `{quick_check}`\n- Automatic clean Stop requested: {auto_stop}\n- Automatic clean Stop error: `{}`\n- Final capture state: `{}`\n- Final event queue: {}/{}\n- Final workload queue: {}/{}\n- Continue requests during run: {}\n- Continue decision row growth: {}\n- New helper abnormal exits: {}\n- New bounded helper timeouts: {}\n- Unhandled or unreaped helper timeouts: {}\n- Last helper operation: `{}`\n- Last helper error category: `{}`\n- New schema initializations after harness start: {}\n- Final status p95: {} microseconds\n- Final status bytes: {}\n\nThis report is measurement evidence for one scenario. It is not an overall always-on stability declaration.\n",
        duration.as_secs(),
        started.elapsed().as_secs(),
        final_crashes.0.saturating_sub(crash_baseline.0),
        final_crashes.1.saturating_sub(crash_baseline.1),
        metrics.warmup_seconds,
        metrics.post_warmup_sample_count,
        optional_metric(metrics.cpu_p95_percent),
        metrics.cpu_p95_gate_percent,
        optional_gate(metrics.cpu_p95_passed),
        optional_metric(metrics.memory_growth_bytes_per_hour),
        metrics.memory_growth_gate_bytes_per_hour,
        optional_gate(metrics.memory_growth_passed),
        metrics.maximum_event_queue_depth,
        metrics.maximum_event_queue_capacity,
        metrics.maximum_event_queue_high_water_mark,
        metrics.maximum_workload_queue_depth,
        metrics.maximum_workload_queue_capacity,
        metrics.maximum_workload_queue_high_water_mark,
        metrics.maximum_status_p95_latency_us,
        metrics.maximum_status_response_bytes,
        shutdown_error.as_deref().unwrap_or("none"),
        final_sample.as_ref().map(|sample| sample.capture_runtime_state.as_str()).unwrap_or("unavailable"),
        final_sample.as_ref().map(|sample| sample.event_queue_depth).unwrap_or(0),
        final_sample.as_ref().map(|sample| sample.event_queue_capacity).unwrap_or(0),
        final_sample.as_ref().map(|sample| sample.workload_queue_depth).unwrap_or(0),
        final_sample.as_ref().map(|sample| sample.workload_queue_capacity).unwrap_or(0),
        final_sample.as_ref().map(|sample| sample.continue_requests).unwrap_or(0),
        final_sample.as_ref().map(|sample| sample.continue_decision_row_growth).unwrap_or(0),
        final_sample.as_ref().map(|sample| sample.helper_abnormal_exits).unwrap_or(0),
        final_sample.as_ref().map(|sample| sample.helper_timeouts).unwrap_or(0),
        final_sample.as_ref().map(|sample| sample.unhandled_helper_timeouts).unwrap_or(0),
        final_sample.as_ref().and_then(|sample| sample.last_helper_operation.as_deref()).unwrap_or("none"),
        final_sample.as_ref().and_then(|sample| sample.last_helper_error_category.as_deref()).unwrap_or("none"),
        final_sample.as_ref().map(|sample| sample.schema_initialization_count).unwrap_or(0),
        final_sample.as_ref().map(|sample| sample.status_p95_latency_us).unwrap_or(0),
        final_sample.as_ref().map(|sample| sample.status_response_bytes).unwrap_or(0),
    );
    let _ = fs::write(root.join("summary.md"), summary);
}

#[cfg(debug_assertions)]
fn build_metrics(samples: &[SoakSample], duration: Duration) -> SoakMetrics {
    let warmup = Duration::from_secs((duration.as_secs() / 5).min(10 * 60));
    let warmup_ms = warmup.as_millis().min(u64::MAX as u128) as u64;
    let post_warmup = samples
        .iter()
        .filter(|sample| {
            sample.elapsed_ms >= warmup_ms
                && sample.main_process_alive
                && sample.capture_runtime_state != "stopped"
        })
        .collect::<Vec<_>>();
    let mut cpu = post_warmup
        .iter()
        .filter_map(|sample| sample.cpu_percent)
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    let memory = post_warmup
        .iter()
        .filter_map(|sample| {
            sample
                .resident_memory_bytes
                .map(|bytes| (sample.elapsed_ms as f64, bytes as f64))
        })
        .collect::<Vec<_>>();
    let cpu_gate = policy_threshold_f64("quiet_cpu_p95_percent").unwrap_or(10.0);
    let memory_gate =
        policy_threshold_f64("memory_growth_after_warmup_bytes_per_hour").unwrap_or(0.0);
    let cpu_p95 = percentile_nearest_rank(&mut cpu, 0.95);
    let memory_growth = linear_growth_per_hour(&memory);

    SoakMetrics {
        schema: "smalltalk.runtime_soak_metrics.v1",
        warmup_seconds: warmup.as_secs(),
        post_warmup_sample_count: post_warmup.len() as u64,
        cpu_p95_percent: cpu_p95,
        cpu_p95_gate_percent: cpu_gate,
        cpu_p95_passed: cpu_p95.map(|value| value <= cpu_gate),
        memory_growth_bytes_per_hour: memory_growth,
        memory_growth_gate_bytes_per_hour: memory_gate,
        memory_growth_passed: memory_growth.map(|value| value <= memory_gate),
        maximum_event_queue_depth: samples
            .iter()
            .map(|sample| sample.event_queue_depth)
            .max()
            .unwrap_or(0),
        maximum_event_queue_capacity: samples
            .iter()
            .map(|sample| sample.event_queue_capacity)
            .max()
            .unwrap_or(0),
        maximum_event_queue_high_water_mark: samples
            .iter()
            .map(|sample| sample.event_queue_high_water_mark)
            .max()
            .unwrap_or(0),
        maximum_workload_queue_depth: samples
            .iter()
            .map(|sample| sample.workload_queue_depth)
            .max()
            .unwrap_or(0),
        maximum_workload_queue_capacity: samples
            .iter()
            .map(|sample| sample.workload_queue_capacity)
            .max()
            .unwrap_or(0),
        maximum_workload_queue_high_water_mark: samples
            .iter()
            .map(|sample| sample.workload_queue_high_water_mark)
            .max()
            .unwrap_or(0),
        maximum_status_p95_latency_us: samples
            .iter()
            .map(|sample| sample.status_p95_latency_us)
            .max()
            .unwrap_or(0),
        maximum_status_response_bytes: samples
            .iter()
            .map(|sample| sample.status_response_bytes)
            .max()
            .unwrap_or(0),
    }
}

#[cfg(debug_assertions)]
fn policy_threshold_f64(key: &str) -> Option<f64> {
    serde_json::from_str::<Value>(POLICY_JSON)
        .ok()?
        .get("thresholds")?
        .get(key)?
        .as_f64()
}

#[cfg(debug_assertions)]
fn percentile_nearest_rank(values: &mut [f64], percentile: f64) -> Option<f64> {
    if values.is_empty() || !(0.0..=1.0).contains(&percentile) {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let rank = (percentile * values.len() as f64).ceil().max(1.0) as usize;
    values.get(rank.saturating_sub(1)).copied()
}

#[cfg(debug_assertions)]
fn linear_growth_per_hour(samples: &[(f64, f64)]) -> Option<f64> {
    if samples.len() < 2 {
        return None;
    }
    let count = samples.len() as f64;
    let mean_x = samples.iter().map(|(x, _)| x).sum::<f64>() / count;
    let mean_y = samples.iter().map(|(_, y)| y).sum::<f64>() / count;
    let denominator = samples
        .iter()
        .map(|(x, _)| (x - mean_x).powi(2))
        .sum::<f64>();
    if denominator <= f64::EPSILON {
        return None;
    }
    let numerator = samples
        .iter()
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum::<f64>();
    Some((numerator / denominator) * 60.0 * 60.0 * 1_000.0)
}

#[cfg(debug_assertions)]
fn optional_metric(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "unavailable".to_string())
}

#[cfg(debug_assertions)]
fn optional_gate(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unavailable",
    }
}

#[cfg(debug_assertions)]
fn collect_sample(
    app: &AppHandle,
    run_id: &str,
    scenario: &str,
    elapsed: Duration,
    process_start_count: u64,
) -> Option<SoakSample> {
    let state = app.state::<CaptureState>();
    let status = capture_status_snapshot_inner(app, &state.inner).ok()?;
    let diagnostics = &status.runtime_diagnostics;
    let paths = capture_paths(app).ok()?;
    let conn = open_db_at_path(&paths.db_path, false).ok();
    let mut row_counts = HashMap::new();
    if let Some(conn) = conn.as_ref() {
        for table in [
            "capture_sessions",
            "frames",
            "ui_events",
            "capture_triggers",
            "event_transitions",
            "typing_bursts",
            "continue_decisions",
            "continue_candidates",
            "continue_task_actions",
        ] {
            if let Ok(count) = row_count(conn, table) {
                row_counts.insert(table.to_string(), count);
            }
        }
    }
    let (cpu_percent, resident_memory_bytes, thread_count) = process_metrics();
    let db_bytes = fs::metadata(&paths.db_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let wal_bytes = fs::metadata(format!("{}-wal", paths.db_path.to_string_lossy()))
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let (main_crash_reports, helper_crash_reports) = crash_report_counts();
    Some(SoakSample {
        schema: "smalltalk.runtime_soak_sample.v1",
        run_id: run_id.to_string(),
        scenario: scenario.to_string(),
        sampled_at_ms: now_millis(),
        elapsed_ms: elapsed.as_millis().min(u64::MAX as u128) as u64,
        main_process_alive: true,
        process_start_count,
        cpu_percent,
        resident_memory_bytes,
        thread_count,
        file_descriptor_count: fs::read_dir("/dev/fd")
            .ok()
            .map(|entries| entries.count() as u64),
        capture_runtime_state: diagnostics.capture_runtime_state.clone(),
        session_state: status
            .active_session
            .as_ref()
            .or(status.latest_session.as_ref())
            .map(|session| session.status.clone()),
        helper_launches: diagnostics.helper_launches,
        helper_abnormal_exits: diagnostics.helper_abnormal_exits,
        helper_timeouts: diagnostics.helper_timeouts,
        helper_timeouts_reaped: diagnostics.helper_timeouts_reaped,
        unhandled_helper_timeouts: diagnostics
            .helper_timeouts
            .saturating_sub(diagnostics.helper_timeouts_reaped),
        helper_cancellations: diagnostics.helper_cancellations,
        active_child_processes: diagnostics.active_child_processes,
        current_helper_operation: diagnostics.current_operation_class.clone(),
        last_helper_operation: diagnostics.last_operation_class.clone(),
        last_helper_duration_ms: diagnostics.last_operation_duration_ms,
        last_helper_error_category: diagnostics.last_safe_error_category.clone(),
        event_queue_depth: diagnostics.event_pipeline.queue_depth,
        event_queue_capacity: diagnostics.event_pipeline.queue_capacity,
        event_queue_high_water_mark: diagnostics.event_pipeline.high_water_mark,
        event_coalesced: diagnostics.event_pipeline.coalesced_count,
        event_dropped: diagnostics.event_pipeline.dropped_count,
        workload_active_count: diagnostics.workload.active_operations.len() as u64,
        workload_queue_depth: diagnostics.workload.queued_operation_count,
        workload_queue_capacity: diagnostics.workload.queue_capacity,
        workload_queue_high_water_mark: diagnostics.workload.queue_high_water_mark,
        workload_cancelled_or_superseded: diagnostics.workload.cancelled_or_superseded_requests,
        audit_active: diagnostics.audit_executor.active,
        audit_queued: diagnostics.audit_executor.queued,
        audit_completed: diagnostics.audit_executor.completed,
        audit_failed: diagnostics.audit_executor.failed,
        database_busy_time_ms: diagnostics.database_busy_time_ms,
        database_busy_retry_count: diagnostics.database_busy_retry_count,
        schema_initialization_count: diagnostics.schema_initialization_count,
        database_generation: diagnostics.database_generation,
        database_bytes: db_bytes,
        wal_bytes,
        snapshot_bytes: directory_stats(&paths.snapshot_dir)
            .map(|stats| stats.byte_size.max(0) as u64)
            .unwrap_or(0),
        audit_bytes: project_continue_outputs_root()
            .ok()
            .and_then(|path| directory_stats(&path).ok())
            .map(|stats| stats.byte_size.max(0) as u64)
            .unwrap_or(0),
        row_counts,
        capture_stores: diagnostics.heavy_captures_stored,
        capture_skips: diagnostics.heavy_captures_skipped,
        ocr_attempts: diagnostics.ocr_runs,
        continue_requests: diagnostics
            .continue_normal_calls
            .saturating_add(diagnostics.continue_rebuild_calls),
        continue_cache_hits: diagnostics.decision_cache_hits,
        continue_decision_row_growth: 0,
        status_p50_latency_us: diagnostics.status_metrics.p50_latency_us,
        status_p95_latency_us: diagnostics.status_metrics.p95_latency_us,
        status_response_bytes: diagnostics.status_metrics.last_response_bytes,
        stop_latency_ms: diagnostics.stop_latency_ms,
        main_crash_reports,
        helper_crash_reports,
    })
}

#[cfg(debug_assertions)]
fn append_json_line(path: &Path, sample: &SoakSample) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(to_string)?;
    serde_json::to_writer(&mut file, sample).map_err(to_string)?;
    file.write_all(b"\n").map_err(to_string)
}

#[cfg(debug_assertions)]
fn process_metrics() -> (Option<f64>, Option<u64>, Option<u64>) {
    let pid = std::process::id().to_string();
    let output = Command::new("/bin/ps")
        .args(["-p", &pid, "-o", "%cpu=,rss="])
        .output()
        .ok();
    let fields = output
        .as_ref()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout.clone()).ok())
        .map(|line| {
            line.split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let thread_count = Command::new("/bin/ps")
        .args(["-M", "-p", &pid])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|output| output.lines().count().saturating_sub(2) as u64);
    (
        fields.first().and_then(|value| value.parse::<f64>().ok()),
        fields
            .get(1)
            .and_then(|value| value.parse::<u64>().ok())
            .map(|kilobytes| kilobytes.saturating_mul(1024)),
        thread_count,
    )
}

#[cfg(debug_assertions)]
fn crash_report_counts() -> (u64, u64) {
    let Some(home) = std::env::var_os("HOME") else {
        return (0, 0);
    };
    let reports = PathBuf::from(home).join("Library/Logs/DiagnosticReports");
    let Ok(entries) = fs::read_dir(reports) else {
        return (0, 0);
    };
    let mut main = 0_u64;
    let mut helper = 0_u64;
    for name in entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
    {
        if name.starts_with("sck_screenshot")
            && (name.ends_with(".ips") || name.ends_with(".crash"))
        {
            helper = helper.saturating_add(1);
        } else if name.starts_with("smalltalk")
            && (name.ends_with(".ips") || name.ends_with(".crash"))
        {
            main = main.saturating_add(1);
        }
    }
    (main, helper)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versioned_runtime_policy_parses_and_keeps_absolute_gates() {
        let policy: Value = serde_json::from_str(POLICY_JSON).unwrap();
        assert_eq!(policy["schema"], "smalltalk.runtime_stability_policy.v1");
        assert_eq!(policy["thresholds"]["new_main_process_crash_reports"], 0);
        assert_eq!(
            policy["thresholds"]["event_queue_capacity"],
            TOTAL_EVENT_QUEUE_CAPACITY
        );
        assert_eq!(policy["thresholds"]["workload_queue_capacity"], 48);
    }

    #[test]
    fn soak_counters_report_only_changes_after_the_run_baseline() {
        let baseline_sample = SoakSample {
            helper_launches: 10,
            helper_timeouts: 2,
            helper_timeouts_reaped: 2,
            schema_initialization_count: 4,
            continue_requests: 20,
            row_counts: HashMap::from([("continue_decisions".to_string(), 7)]),
            ..SoakSample::default()
        };
        let baseline = CounterBaseline::from_sample(&baseline_sample);
        let mut later = SoakSample {
            helper_launches: 13,
            helper_timeouts: 2,
            helper_timeouts_reaped: 2,
            schema_initialization_count: 4,
            continue_requests: 22,
            row_counts: HashMap::from([("continue_decisions".to_string(), 8)]),
            ..SoakSample::default()
        };

        baseline.apply(&mut later);

        assert_eq!(later.helper_launches, 3);
        assert_eq!(later.helper_timeouts, 0);
        assert_eq!(later.unhandled_helper_timeouts, 0);
        assert_eq!(later.schema_initialization_count, 0);
        assert_eq!(later.continue_requests, 2);
        assert_eq!(later.continue_decision_row_growth, 1);
    }

    #[test]
    fn soak_auto_start_is_explicit_and_requires_a_scenario() {
        assert!(auto_start_capture_value(true, Some("1")));
        assert!(auto_start_capture_value(true, Some("YES")));
        assert!(!auto_start_capture_value(false, Some("1")));
        assert!(!auto_start_capture_value(true, Some("0")));
        assert!(!auto_start_capture_value(true, None));
    }

    #[test]
    fn soak_auto_stop_uses_the_same_explicit_boolean_contract() {
        assert!(harness_boolean_value(true, Some("on")));
        assert!(harness_boolean_value(true, Some("TRUE")));
        assert!(!harness_boolean_value(false, Some("true")));
        assert!(!harness_boolean_value(true, Some("off")));
    }

    #[test]
    fn soak_metrics_use_nearest_rank_and_linear_post_warmup_growth() {
        let mut values = vec![1.0, 3.0, 2.0, 100.0, 4.0];
        assert_eq!(percentile_nearest_rank(&mut values, 0.80), Some(4.0));
        assert_eq!(percentile_nearest_rank(&mut values, 0.95), Some(100.0));

        let growth =
            linear_growth_per_hour(&[(0.0, 1_000.0), (30_000.0, 2_000.0), (60_000.0, 3_000.0)])
                .unwrap();
        assert!((growth - 120_000.0).abs() < 0.01);
    }

    const TOTAL_EVENT_QUEUE_CAPACITY: u64 = event_pipeline::TOTAL_CAPACITY as u64;
}
