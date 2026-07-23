use rusqlite::Connection;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

const REPORT_SCHEMA: &str = "smalltalk.mfti_04.performance_cost_privacy.v1";
const SAMPLE_SCHEMA: &str = "smalltalk.mfti_04.performance_sample.v1";
const MIN_COMPLETE_SAMPLES: usize = 30;

#[derive(Debug, Clone)]
struct Options {
    database: PathBuf,
    output: PathBuf,
    monthly_continues: u64,
    privacy_violations: u64,
    unsafe_opens: u64,
    provider_failure_experience_reviewed: bool,
}

#[derive(Debug, Clone)]
struct Sample {
    capture_to_packet_ms: f64,
    request_build_ms: f64,
    provider_ms: f64,
    verification_persistence_ms: f64,
    total_manual_continue_ms: f64,
    image_count: f64,
    image_bytes: f64,
    input_tokens: f64,
    output_tokens: f64,
    estimated_cost_usd: f64,
    provider_outcome: String,
    second_pass_ran: bool,
    second_pass_cost_usd: f64,
    privacy_excluded_frame_count: u64,
    transported_frame_count: u64,
    privacy_blocked_before_transport: bool,
    background_multimodal_requests: u64,
}

#[derive(Debug, Serialize)]
struct Measurements {
    capture_to_packet_p50_ms: f64,
    capture_to_packet_p95_ms: f64,
    request_build_p50_ms: f64,
    request_build_p95_ms: f64,
    provider_p50_ms: f64,
    provider_p95_ms: f64,
    verification_persistence_p50_ms: f64,
    verification_persistence_p95_ms: f64,
    manual_continue_p50_ms: f64,
    manual_continue_p95_ms: f64,
    image_count_p50: f64,
    image_count_p95: f64,
    image_bytes_p50: f64,
    image_bytes_p95: f64,
    input_tokens_p50: f64,
    input_tokens_p95: f64,
    output_tokens_p50: f64,
    output_tokens_p95: f64,
    cost_per_continue_usd: f64,
    expected_monthly_cost_usd: f64,
    provider_timeout_rate: f64,
    provider_error_rate: f64,
    invalid_output_rate: f64,
    second_pass_rate: f64,
    second_pass_cost_usd: f64,
    privacy_exclusion_rate: f64,
}

#[derive(Debug, Serialize)]
struct MeasurementPolicy {
    complete_sample_minimum: usize,
    monthly_continues: u64,
    input_cost_usd_per_million_tokens: f64,
    output_cost_usd_per_million_tokens: f64,
    input_token_source: &'static str,
    output_token_source: &'static str,
    source_data_policy: &'static str,
}

#[derive(Debug, Serialize)]
struct Report {
    schema: &'static str,
    policy_frozen_before_holdout_access: bool,
    holdout_accessed_at_freeze: bool,
    sample_count: usize,
    measurement_policy: MeasurementPolicy,
    measurements: Measurements,
    background_multimodal_requests: u64,
    privacy_blocked_frames_excluded_before_transport: bool,
    privacy_violations: u64,
    unsafe_opens: u64,
    provider_failure_experience_reviewed: bool,
}

fn parse_u64(value: Option<String>, flag: &str) -> Result<u64, String> {
    value
        .ok_or_else(|| format!("{flag} requires a value"))?
        .parse::<u64>()
        .map_err(|_| format!("{flag} must be a non-negative integer"))
}

fn parse_options() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let mut database = None;
    let mut output = None;
    let mut monthly_continues = None;
    let mut privacy_violations = None;
    let mut unsafe_opens = None;
    let mut provider_failure_experience_reviewed = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--database" => database = args.next().map(PathBuf::from),
            "--output" => output = args.next().map(PathBuf::from),
            "--monthly-continues" => {
                monthly_continues = Some(parse_u64(args.next(), "--monthly-continues")?)
            }
            "--privacy-violations" => {
                privacy_violations = Some(parse_u64(args.next(), "--privacy-violations")?)
            }
            "--unsafe-opens" => unsafe_opens = Some(parse_u64(args.next(), "--unsafe-opens")?),
            "--provider-failure-experience-reviewed" => provider_failure_experience_reviewed = true,
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    let monthly_continues =
        monthly_continues.ok_or_else(|| "--monthly-continues is required".to_string())?;
    if monthly_continues == 0 {
        return Err("--monthly-continues must be at least 1".to_string());
    }
    Ok(Options {
        database: database.ok_or_else(|| "--database is required".to_string())?,
        output: output.ok_or_else(|| "--output is required".to_string())?,
        monthly_continues,
        privacy_violations: privacy_violations
            .ok_or_else(|| "--privacy-violations is required".to_string())?,
        unsafe_opens: unsafe_opens.ok_or_else(|| "--unsafe-opens is required".to_string())?,
        provider_failure_experience_reviewed,
    })
}

fn load_complete_samples(conn: &Connection) -> Result<Vec<Sample>, String> {
    let mut statement = conn
        .prepare(
            "SELECT capture_to_packet_ms, request_build_ms, provider_ms,
                    verification_persistence_ms, total_manual_continue_ms,
                    image_count, image_bytes, input_tokens, output_tokens,
                    estimated_cost_usd, provider_outcome, second_pass_ran,
                    second_pass_cost_usd, privacy_excluded_frame_count,
                    transported_frame_count, privacy_blocked_before_transport,
                    background_multimodal_requests
             FROM task_truth_v2_performance_samples
             WHERE sample_complete=1 AND schema_version=?1
             ORDER BY observed_at_ms ASC, sample_id ASC",
        )
        .map_err(|error| format!("performance sample table unavailable: {error}"))?;
    let samples = statement
        .query_map([SAMPLE_SCHEMA], |row| {
            Ok(Sample {
                capture_to_packet_ms: row.get::<_, i64>(0)?.max(0) as f64,
                request_build_ms: row.get::<_, i64>(1)?.max(0) as f64,
                provider_ms: row.get::<_, i64>(2)?.max(0) as f64,
                verification_persistence_ms: row.get::<_, i64>(3)?.max(0) as f64,
                total_manual_continue_ms: row.get::<_, i64>(4)?.max(0) as f64,
                image_count: row.get::<_, i64>(5)?.max(0) as f64,
                image_bytes: row.get::<_, i64>(6)?.max(0) as f64,
                input_tokens: row.get::<_, i64>(7)?.max(0) as f64,
                output_tokens: row.get::<_, i64>(8)?.max(0) as f64,
                estimated_cost_usd: row.get::<_, f64>(9)?.max(0.0),
                provider_outcome: row.get(10)?,
                second_pass_ran: row.get::<_, i64>(11)? == 1,
                second_pass_cost_usd: row.get::<_, f64>(12)?.max(0.0),
                privacy_excluded_frame_count: row.get::<_, i64>(13)?.max(0) as u64,
                transported_frame_count: row.get::<_, i64>(14)?.max(0) as u64,
                privacy_blocked_before_transport: row.get::<_, i64>(15)? == 1,
                background_multimodal_requests: row.get::<_, i64>(16)?.max(0) as u64,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(samples)
}

fn percentile(mut values: Vec<f64>, quantile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    let index = (((values.len() - 1) as f64) * quantile).ceil() as usize;
    values[index.min(values.len() - 1)]
}

fn rate(count: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        count as f64 / denominator as f64
    }
}

fn mean(values: impl Iterator<Item = f64>, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        values.sum::<f64>() / denominator as f64
    }
}

fn build_report(samples: &[Sample], options: &Options) -> Report {
    let count = samples.len();
    let p = |field: fn(&Sample) -> f64, quantile| {
        percentile(samples.iter().map(field).collect::<Vec<_>>(), quantile)
    };
    let cost_per_continue = mean(
        samples.iter().map(|sample| sample.estimated_cost_usd),
        count,
    );
    let excluded = samples
        .iter()
        .map(|sample| sample.privacy_excluded_frame_count)
        .sum::<u64>();
    let transported = samples
        .iter()
        .map(|sample| sample.transported_frame_count)
        .sum::<u64>();
    let privacy_denominator = excluded.saturating_add(transported);
    Report {
        schema: REPORT_SCHEMA,
        policy_frozen_before_holdout_access: true,
        holdout_accessed_at_freeze: false,
        sample_count: count,
        measurement_policy: MeasurementPolicy {
            complete_sample_minimum: MIN_COMPLETE_SAMPLES,
            monthly_continues: options.monthly_continues,
            input_cost_usd_per_million_tokens: 0.5,
            output_cost_usd_per_million_tokens: 2.0,
            input_token_source: "provider_actual_or_request_estimate",
            output_token_source: "provider_actual_or_zero_when_no_response",
            source_data_policy: "numeric_and_bounded_categorical_audit_metadata_only",
        },
        measurements: Measurements {
            capture_to_packet_p50_ms: p(|sample| sample.capture_to_packet_ms, 0.50),
            capture_to_packet_p95_ms: p(|sample| sample.capture_to_packet_ms, 0.95),
            request_build_p50_ms: p(|sample| sample.request_build_ms, 0.50),
            request_build_p95_ms: p(|sample| sample.request_build_ms, 0.95),
            provider_p50_ms: p(|sample| sample.provider_ms, 0.50),
            provider_p95_ms: p(|sample| sample.provider_ms, 0.95),
            verification_persistence_p50_ms: {
                p(|sample| sample.verification_persistence_ms, 0.50)
            },
            verification_persistence_p95_ms: {
                p(|sample| sample.verification_persistence_ms, 0.95)
            },
            manual_continue_p50_ms: p(|sample| sample.total_manual_continue_ms, 0.50),
            manual_continue_p95_ms: p(|sample| sample.total_manual_continue_ms, 0.95),
            image_count_p50: p(|sample| sample.image_count, 0.50),
            image_count_p95: p(|sample| sample.image_count, 0.95),
            image_bytes_p50: p(|sample| sample.image_bytes, 0.50),
            image_bytes_p95: p(|sample| sample.image_bytes, 0.95),
            input_tokens_p50: p(|sample| sample.input_tokens, 0.50),
            input_tokens_p95: p(|sample| sample.input_tokens, 0.95),
            output_tokens_p50: p(|sample| sample.output_tokens, 0.50),
            output_tokens_p95: p(|sample| sample.output_tokens, 0.95),
            cost_per_continue_usd: cost_per_continue,
            expected_monthly_cost_usd: cost_per_continue * options.monthly_continues as f64,
            provider_timeout_rate: rate(
                samples
                    .iter()
                    .filter(|sample| sample.provider_outcome == "timeout")
                    .count(),
                count,
            ),
            provider_error_rate: rate(
                samples
                    .iter()
                    .filter(|sample| {
                        matches!(
                            sample.provider_outcome.as_str(),
                            "provider_error" | "model_unavailable" | "credentials_missing"
                        )
                    })
                    .count(),
                count,
            ),
            invalid_output_rate: rate(
                samples
                    .iter()
                    .filter(|sample| {
                        matches!(
                            sample.provider_outcome.as_str(),
                            "invalid_response" | "verification_rejected" | "request_invalid"
                        )
                    })
                    .count(),
                count,
            ),
            second_pass_rate: rate(
                samples
                    .iter()
                    .filter(|sample| sample.second_pass_ran)
                    .count(),
                count,
            ),
            second_pass_cost_usd: mean(
                samples.iter().map(|sample| sample.second_pass_cost_usd),
                count,
            ),
            privacy_exclusion_rate: if privacy_denominator == 0 {
                0.0
            } else {
                excluded as f64 / privacy_denominator as f64
            },
        },
        background_multimodal_requests: samples
            .iter()
            .map(|sample| sample.background_multimodal_requests)
            .sum(),
        privacy_blocked_frames_excluded_before_transport: count > 0
            && samples
                .iter()
                .all(|sample| sample.privacy_blocked_before_transport),
        privacy_violations: options.privacy_violations,
        unsafe_opens: options.unsafe_opens,
        provider_failure_experience_reviewed: options.provider_failure_experience_reviewed,
    }
}

fn write_report(path: &Path, report: &Report) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(report).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn run() -> Result<(), String> {
    let options = parse_options()?;
    let conn = Connection::open(&options.database).map_err(|error| error.to_string())?;
    let samples = load_complete_samples(&conn)?;
    let report = build_report(&samples, &options);
    write_report(&options.output, &report)?;
    if report.sample_count < MIN_COMPLETE_SAMPLES {
        eprintln!(
            "wrote provisional MFTI-04 report: {} complete samples; {} required for release",
            report.sample_count, MIN_COMPLETE_SAMPLES
        );
    } else {
        eprintln!(
            "wrote MFTI-04 report from {} complete privacy-safe samples",
            report.sample_count
        );
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("MFTI-04 performance report failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Options {
        Options {
            database: PathBuf::from("unused"),
            output: PathBuf::from("unused"),
            monthly_continues: 600,
            privacy_violations: 0,
            unsafe_opens: 0,
            provider_failure_experience_reviewed: true,
        }
    }

    fn sample(outcome: &str) -> Sample {
        Sample {
            capture_to_packet_ms: 10.0,
            request_build_ms: 20.0,
            provider_ms: 100.0,
            verification_persistence_ms: 30.0,
            total_manual_continue_ms: 160.0,
            image_count: 2.0,
            image_bytes: 4000.0,
            input_tokens: 800.0,
            output_tokens: 200.0,
            estimated_cost_usd: 0.0008,
            provider_outcome: outcome.to_string(),
            second_pass_ran: false,
            second_pass_cost_usd: 0.0,
            privacy_excluded_frame_count: 1,
            transported_frame_count: 2,
            privacy_blocked_before_transport: true,
            background_multimodal_requests: 0,
        }
    }

    #[test]
    fn report_requires_thirty_complete_samples_for_release_denominator() {
        let report = build_report(&vec![sample("success"); 29], &options());
        assert_eq!(report.sample_count, 29);
        assert!(report.sample_count < MIN_COMPLETE_SAMPLES);
        let report = build_report(&vec![sample("success"); 30], &options());
        assert_eq!(report.sample_count, 30);
    }

    #[test]
    fn loader_ignores_incomplete_samples_and_output_has_no_semantic_payload() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE task_truth_v2_performance_samples (
               sample_id TEXT PRIMARY KEY, schema_version TEXT, observed_at_ms INTEGER,
               sample_complete INTEGER, capture_to_packet_ms INTEGER, request_build_ms INTEGER,
               provider_ms INTEGER, verification_persistence_ms INTEGER,
               total_manual_continue_ms INTEGER, image_count INTEGER, image_bytes INTEGER,
               input_tokens INTEGER, output_tokens INTEGER, estimated_cost_usd REAL,
               provider_outcome TEXT, second_pass_ran INTEGER, second_pass_cost_usd REAL,
               privacy_excluded_frame_count INTEGER, transported_frame_count INTEGER,
               privacy_blocked_before_transport INTEGER, background_multimodal_requests INTEGER
             );
             INSERT INTO task_truth_v2_performance_samples VALUES
               ('complete','smalltalk.mfti_04.performance_sample.v1',1,1,10,20,100,30,160,2,4000,800,200,0.0008,'success',0,0,1,2,1,0),
               ('incomplete','smalltalk.mfti_04.performance_sample.v1',2,0,999,999,999,999,999,9,9999,999,999,9.0,'provider_error',1,9.0,9,9,0,9);",
        )
        .unwrap();
        let samples = load_complete_samples(&conn).unwrap();
        assert_eq!(samples.len(), 1);
        let encoded = serde_json::to_string(&build_report(&samples, &options())).unwrap();
        for forbidden in [
            "ocr",
            "accessibility",
            "hypothesis",
            "/Users/",
            "raw_response",
        ] {
            assert!(!encoded.to_ascii_lowercase().contains(forbidden));
        }
    }

    #[test]
    fn rates_and_monthly_cost_use_complete_continue_denominator() {
        let samples = vec![sample("success"), sample("timeout")];
        let report = build_report(&samples, &options());
        assert_eq!(report.measurements.provider_timeout_rate, 0.5);
        assert_eq!(report.measurements.cost_per_continue_usd, 0.0008);
        assert!((report.measurements.expected_monthly_cost_usd - 0.48).abs() < 1e-12);
        assert_eq!(report.measurements.privacy_exclusion_rate, 1.0 / 3.0);
    }
}
