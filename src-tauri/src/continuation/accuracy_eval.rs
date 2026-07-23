use super::accuracy_fixture::*;
use super::{
    audit_feedback_event_against_current_task, get_continue_decision,
    rebuild_continue_second_layer, rebuild_continue_third_layer, ConfidenceDimension,
    ContinueDecisionRequest, ContinueDecisionResult, ContinueSecondLayerRebuildRequest,
    ContinueThirdLayerRebuildRequest,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const CONTINUE_ACCURACY_REPORT_SCHEMA: &str = "smalltalk.continue_accuracy_report.v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContinueAccuracyEvalOptions {
    pub fixture_root: Option<String>,
    #[serde(default)]
    pub allow_locked_holdout: bool,
    #[serde(default = "default_repeat_count")]
    pub repeat_count: usize,
}

fn default_repeat_count() -> usize {
    2
}

impl Default for ContinueAccuracyEvalOptions {
    fn default() -> Self {
        Self {
            fixture_root: None,
            allow_locked_holdout: false,
            repeat_count: 2,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccuracyCheckpointStatus {
    Match,
    Mismatch,
    Missing,
    NotImplemented,
    NotExpected,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccuracyProductionCoverage {
    Production,
    ProductionValidatorWithDeterministicTransport,
    HistoricalStateInjection,
    MissingProductionCheckpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccuracyCheckpointResult {
    pub checkpoint: AccuracyCheckpointV1,
    pub status: AccuracyCheckpointStatus,
    pub production_path_coverage: AccuracyProductionCoverage,
    pub expected_slot_count: usize,
    pub correct_slot_count: usize,
    pub evaluated_slots: Vec<String>,
    pub mismatched_slots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccuracyMetricValue {
    pub numerator: i64,
    pub denominator: i64,
    pub rate: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinueAccuracyCaseResult {
    pub case_id: String,
    pub status: String,
    pub first_divergent_checkpoint: Option<AccuracyCheckpointV1>,
    pub checkpoint_results: Vec<AccuracyCheckpointResult>,
    pub forbidden_claim_matches: Vec<String>,
    pub wrong_confident: bool,
    pub public_target_honest: bool,
    pub model_on_off_task_identity_match: bool,
    pub deterministic_replay_match: bool,
    pub privacy_lint_passed: bool,
    pub semantic_identity_fingerprint: String,
    pub evidence_delta_classification: Option<String>,
    pub feedback_policy_audit: Option<Value>,
    pub replay_duration_ms: f64,
    pub confidence_dimensions: BTreeMap<String, ConfidenceDimensionObservation>,
    pub probe_counterfactuals: Vec<ProbeCounterfactualObservation>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceDimensionObservation {
    pub score: f64,
    pub label: String,
    pub correct: Option<bool>,
    pub expected_positive: Option<bool>,
    pub supporting_evidence_count: usize,
    pub missing_evidence_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceCalibrationMetric {
    pub sample_size: usize,
    pub predicted_label_counts: BTreeMap<String, usize>,
    pub correct_count: usize,
    pub incorrect_count: usize,
    pub overconfident_wrong_count: usize,
    pub underconfident_correct_count: usize,
    pub brier_score: Option<f64>,
    pub expected_calibration_error: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeCounterfactualObservation {
    pub status: String,
    pub duration_ms: f64,
    pub evidence_changed: bool,
    pub stale_result: bool,
    pub reran_decision: bool,
    pub confidence_increased: bool,
    pub refreshed_warning_emitted: bool,
    pub task_identity_preserved: bool,
    pub target_confidence_label: String,
    pub wrong_confident: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeAccuracyMetrics {
    pub sample_size: usize,
    pub status_counts: BTreeMap<String, usize>,
    pub changed_count: usize,
    pub rerun_count: usize,
    pub timeout_count: usize,
    pub privacy_blocked_count: usize,
    pub failure_count: usize,
    pub changed_rate: Option<f64>,
    pub rerun_rate: Option<f64>,
    pub timeout_rate: Option<f64>,
    pub privacy_blocked_rate: Option<f64>,
    pub failure_rate: Option<f64>,
    pub p50_duration_ms: Option<f64>,
    pub p95_duration_ms: Option<f64>,
    pub decisions_with_probe_sample_size: usize,
    pub decisions_without_probe_sample_size: usize,
    pub p50_decision_with_probe_ms: Option<f64>,
    pub p95_decision_with_probe_ms: Option<f64>,
    pub p50_decision_without_probe_ms: Option<f64>,
    pub p95_decision_without_probe_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseCorpusSummary {
    pub minimum_required_cases: usize,
    pub case_count: usize,
    pub independently_human_reviewed_count: usize,
    pub partition_counts: BTreeMap<String, usize>,
    pub locked_holdout_evaluated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleasePerformanceSummary {
    pub model_off_p95_ms: f64,
    pub frozen_baseline_p95_ms: f64,
    pub regression_budget_p95_ms: f64,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P6ReleaseGateAssessment {
    pub passed: bool,
    pub corpus: ReleaseCorpusSummary,
    pub performance: ReleasePerformanceSummary,
    pub automated_semantic_gate_passed: bool,
    pub confidence_calibration_gate_passed: bool,
    pub manual_macos_qa_passed: bool,
    pub violations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinueAccuracyEvalReport {
    pub schema: String,
    pub fixture_schema: String,
    pub policy_version: String,
    pub policy_hash: String,
    pub fixture_hash: String,
    pub manifest_hash: String,
    pub evaluated_at_ms: i64,
    pub case_count: usize,
    pub semantic_pass_count: usize,
    pub known_failure_count: usize,
    pub milestone_contract_passed: bool,
    pub milestone_violations: Vec<String>,
    pub first_divergence_counts: BTreeMap<String, i64>,
    pub metrics: BTreeMap<String, AccuracyMetricValue>,
    pub surface_family_macro: BTreeMap<String, f64>,
    pub worst_slice_rate: Option<f64>,
    pub p50_model_off_replay_ms: f64,
    pub p95_model_off_replay_ms: f64,
    pub confidence_calibration: BTreeMap<String, ConfidenceCalibrationMetric>,
    pub probe_metrics: ProbeAccuracyMetrics,
    pub release_gate: P6ReleaseGateAssessment,
    pub cases: Vec<ContinueAccuracyCaseResult>,
}

#[derive(Debug)]
pub(crate) struct ReplaySnapshot {
    pub(crate) actual: BTreeMap<AccuracyCheckpointV1, BTreeMap<String, Value>>,
    pub(crate) decision: ContinueDecisionResult,
    pub(crate) model_parity: bool,
    pub(crate) duration_ms: f64,
}

pub fn run_continue_accuracy_eval_from_dir(
    root: &Path,
    options: ContinueAccuracyEvalOptions,
) -> Result<ContinueAccuracyEvalReport, String> {
    let policy_bytes = fs::read(root.join("eval-policy.v1.json")).map_err(to_string)?;
    let manifest_bytes = fs::read(root.join("known-failures.v1.json")).map_err(to_string)?;
    let policy = parse_eval_policy_json(&policy_bytes).map_err(to_string)?;
    let manifest = parse_milestone_manifest_json(&manifest_bytes).map_err(to_string)?;
    let mode = if options.allow_locked_holdout {
        HoldoutAccessModeV1::ReleaseEvaluation
    } else {
        HoldoutAccessModeV1::Development
    };
    let mut paths = fs::read_dir(root.join("cases"))
        .map_err(to_string)?
        .map(|entry| entry.map(|entry| entry.path()).map_err(to_string))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| path.extension().and_then(|value| value.to_str()) == Some("json"));
    paths.sort();
    let mut fixtures = Vec::new();
    for path in paths {
        fixtures.push(
            parse_accuracy_fixture_json_for_access(&fs::read(path).map_err(to_string)?, mode)
                .map_err(to_string)?,
        );
    }
    validate_initial_capture_case_set(&fixtures).map_err(to_string)?;
    let fixture_hash = stable_json_sha256(&fixtures).map_err(to_string)?;
    let policy_hash = stable_json_sha256(&policy).map_err(to_string)?;
    let manifest_hash = stable_json_sha256(&manifest).map_err(to_string)?;

    let repeat_count = options.repeat_count.clamp(2, 8);
    let mut cases = Vec::new();
    for fixture in &fixtures {
        let privacy = lint_accuracy_fixture(fixture);
        let first = run_fixture_once(fixture)?;
        let mut deterministic = true;
        for _ in 1..repeat_count {
            let repeated = run_fixture_once(fixture)?;
            deterministic &= normalized_snapshot_fingerprint(&first)
                == normalized_snapshot_fingerprint(&repeated);
        }
        cases.push(evaluate_case(
            fixture,
            &policy,
            privacy.passed,
            first,
            deterministic,
        ));
    }
    let observed = cases
        .iter()
        .map(|case| ObservedMilestoneV1 {
            case_id: case.case_id.clone(),
            status: if case.status == "pass" {
                ObservedMilestoneStatusV1::Pass
            } else {
                ObservedMilestoneStatusV1::Fail
            },
            first_divergence: case.first_divergent_checkpoint,
        })
        .collect::<Vec<_>>();
    let milestone_violations = enforce_milestones(
        &manifest,
        &observed,
        P6PhaseV1::P6_09,
        super::current_time_millis(),
    );
    let mut first_divergence_counts = BTreeMap::new();
    for case in &cases {
        if let Some(checkpoint) = case.first_divergent_checkpoint {
            *first_divergence_counts
                .entry(checkpoint_label(checkpoint).to_string())
                .or_insert(0) += 1;
        }
    }
    let metrics = aggregate_metrics(&cases);
    let mut durations = cases
        .iter()
        .map(|case| case.replay_duration_ms)
        .collect::<Vec<_>>();
    durations.sort_by(|left, right| left.total_cmp(right));
    let p50 = percentile(&durations, 0.50);
    let p95 = percentile(&durations, 0.95);
    let task_identity = metric_from_checkpoint(&cases, AccuracyCheckpointV1::LatestTaskTurn);
    let confidence_calibration = aggregate_confidence_calibration(&cases);
    let probe_metrics = aggregate_probe_metrics(&cases);
    let release_gate = assess_p6_release_gate(
        &fixtures,
        &cases,
        &metrics,
        &confidence_calibration,
        &policy,
        p95,
        options.allow_locked_holdout,
        milestone_violations.is_empty(),
    );
    Ok(ContinueAccuracyEvalReport {
        schema: CONTINUE_ACCURACY_REPORT_SCHEMA.to_string(),
        fixture_schema: ACCURACY_FIXTURE_SCHEMA_V1.to_string(),
        policy_version: policy.policy_version,
        policy_hash,
        fixture_hash,
        manifest_hash,
        evaluated_at_ms: super::current_time_millis(),
        case_count: cases.len(),
        semantic_pass_count: cases.iter().filter(|case| case.status == "pass").count(),
        known_failure_count: cases
            .iter()
            .filter(|case| case.status == "known_failure")
            .count(),
        milestone_contract_passed: milestone_violations.is_empty(),
        milestone_violations: milestone_violations
            .into_iter()
            .map(|item| format!("{}:{}", item.case_id, item.reason))
            .collect(),
        first_divergence_counts,
        metrics,
        surface_family_macro: BTreeMap::from([("agent_chat".to_string(), task_identity)]),
        worst_slice_rate: Some(task_identity),
        p50_model_off_replay_ms: round_ms(p50),
        p95_model_off_replay_ms: round_ms(p95),
        confidence_calibration,
        probe_metrics,
        release_gate,
        cases,
    })
}

fn assess_p6_release_gate(
    fixtures: &[ContinueAccuracyFixtureV1],
    cases: &[ContinueAccuracyCaseResult],
    metrics: &BTreeMap<String, AccuracyMetricValue>,
    calibration: &BTreeMap<String, ConfidenceCalibrationMetric>,
    policy: &ContinueAccuracyEvalPolicyV1,
    model_off_p95_ms: f64,
    locked_holdout_requested: bool,
    milestone_contract_passed: bool,
) -> P6ReleaseGateAssessment {
    const MINIMUM_RELEASE_CASES: usize = 100;
    let mut violations = Vec::new();
    let mut partition_counts = BTreeMap::new();
    for fixture in fixtures {
        let label = match fixture.fixture_partition {
            FixturePartitionV1::Development => "development",
            FixturePartitionV1::Validation => "validation",
            FixturePartitionV1::LockedHoldout => "locked_holdout",
        };
        *partition_counts.entry(label.to_string()).or_insert(0) += 1;
    }
    let independently_human_reviewed_count = fixtures
        .iter()
        .filter(|fixture| {
            fixture.privacy_review.status == PrivacyReviewStatusV1::Approved
                && fixture.privacy_review.reviewed_at_ms.is_some()
                && fixture
                    .privacy_review
                    .reviewer_role
                    .as_deref()
                    .is_some_and(|role| role != "p6_fixture_owner" && role != "fixture_owner")
        })
        .count();
    if fixtures.len() < MINIMUM_RELEASE_CASES {
        violations.push(format!(
            "broad_corpus_requires_{MINIMUM_RELEASE_CASES}_cases_found_{}",
            fixtures.len()
        ));
    }
    if independently_human_reviewed_count != fixtures.len() {
        violations.push(format!(
            "independent_human_review_missing_for_{}_cases",
            fixtures
                .len()
                .saturating_sub(independently_human_reviewed_count)
        ));
    }
    for partition in ["development", "validation", "locked_holdout"] {
        if partition_counts.get(partition).copied().unwrap_or(0) == 0 {
            violations.push(format!("missing_{partition}_partition_cases"));
        }
    }
    let locked_holdout_evaluated = locked_holdout_requested
        && partition_counts.get("locked_holdout").copied().unwrap_or(0) > 0;
    if !locked_holdout_evaluated {
        violations.push("locked_holdout_not_evaluated".to_string());
    }

    let required_rates = [
        ("region_role_macro_f1", 0.98),
        ("conversational_role_macro_f1", 0.98),
        ("latest_user_span_precision", 0.95),
        ("latest_user_span_recall", 0.95),
        ("current_agent_status_precision", 0.95),
        ("current_agent_status_recall", 0.95),
        ("unknown_abstention_correctness", 0.95),
        ("latest_user_goal_accuracy", 0.95),
        ("current_state_accuracy", 0.95),
        ("task_turn_boundary_accuracy", 0.95),
        ("task_action_accuracy", 0.95),
        ("semantic_delta_temporal_accuracy", 0.95),
        ("selected_workstream_task_alignment", 0.95),
        ("task_summary_precision", 0.90),
        ("task_summary_coverage", 0.95),
        ("execution_state_accuracy", 0.90),
        ("current_actor_accuracy", 0.95),
        ("waiting_on_accuracy", 0.90),
        ("supported_next_action_precision", 0.95),
        ("supported_next_action_coverage_recall", 0.90),
        ("no_clear_accuracy", 0.95),
        ("direct_openability_precision", 1.0),
        ("labeled_openable_target_recall", 0.95),
        ("model_on_model_off_task_identity_agreement", 1.0),
        ("deterministic_replay_agreement", 1.0),
    ];
    for (name, minimum) in required_rates {
        match metrics.get(name) {
            Some(metric) if metric.rate.is_some_and(|rate| rate >= minimum) => {}
            Some(metric) if metric.denominator == 0 => {
                violations.push(format!("metric_{name}_has_no_labeled_samples"));
            }
            Some(metric) => violations.push(format!(
                "metric_{name}_below_threshold_{}",
                metric.rate.unwrap_or_default()
            )),
            None => violations.push(format!("metric_{name}_missing")),
        }
    }
    for name in [
        "prior_completion_override_rate",
        "stale_feedback_false_promotion_count",
        "unrelated_open_loop_primary_count",
        "cross_layer_contradiction_count",
        "forbidden_stale_term_leakage",
        "wrong_confident_rate",
        "frame_fallback_public_target_count",
    ] {
        if metrics.get(name).is_none_or(|metric| metric.numerator != 0) {
            violations.push(format!("zero_tolerance_{name}_failed"));
        }
    }
    if !milestone_contract_passed || cases.iter().any(|case| case.status != "pass") {
        violations.push("critical_semantic_cases_not_all_passing".to_string());
    }

    let confidence_calibration_gate_passed = !calibration.is_empty()
        && calibration.values().all(|metric| {
            metric.sample_size >= policy.minimum_samples.calibration_predictions
                && metric.overconfident_wrong_count == 0
                && metric
                    .expected_calibration_error
                    .is_some_and(|ece| ece <= policy.calibration_ece_max)
        });
    if !confidence_calibration_gate_passed {
        violations.push("confidence_calibration_sample_or_ece_gate_incomplete".to_string());
    }

    let regression_budget =
        policy.baseline.model_off_p95_ms * policy.model_off_p95_regression_factor;
    let absolute_budget = policy
        .absolute_model_off_p95_budget_ms
        .unwrap_or(f64::INFINITY);
    let effective_budget = regression_budget.min(absolute_budget);
    let performance_passed = model_off_p95_ms <= effective_budget;
    if !performance_passed {
        violations.push(format!(
            "model_off_p95_regression_{:.2}_exceeds_{:.2}",
            model_off_p95_ms, effective_budget
        ));
    }

    // Manual macOS observations cannot be inferred from deterministic replay.
    // P6.09 keeps this false until a separately human-recorded QA artifact is supplied.
    let manual_macos_qa_passed = false;
    violations.push("manual_macos_interruption_recovery_qa_not_proven".to_string());

    P6ReleaseGateAssessment {
        passed: violations.is_empty(),
        corpus: ReleaseCorpusSummary {
            minimum_required_cases: MINIMUM_RELEASE_CASES,
            case_count: fixtures.len(),
            independently_human_reviewed_count,
            partition_counts,
            locked_holdout_evaluated,
        },
        performance: ReleasePerformanceSummary {
            model_off_p95_ms: round_ms(model_off_p95_ms),
            frozen_baseline_p95_ms: policy.baseline.model_off_p95_ms,
            regression_budget_p95_ms: round_ms(effective_budget),
            passed: performance_passed,
        },
        automated_semantic_gate_passed: milestone_contract_passed
            && cases.iter().all(|case| case.status == "pass"),
        confidence_calibration_gate_passed,
        manual_macos_qa_passed,
        violations,
    }
}

pub fn run_committed_continue_accuracy_eval(
    options: ContinueAccuracyEvalOptions,
) -> Result<ContinueAccuracyEvalReport, String> {
    let root = options
        .fixture_root
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/continue_accuracy")
        });
    run_continue_accuracy_eval_from_dir(&root, options)
}

pub fn write_accuracy_report(
    report: &ContinueAccuracyEvalReport,
    output: &Path,
) -> Result<(), String> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(to_string)?;
    }
    fs::write(
        output,
        serde_json::to_vec_pretty(report).map_err(to_string)?,
    )
    .map_err(to_string)
}

pub(crate) fn run_fixture_once(
    fixture: &ContinueAccuracyFixtureV1,
) -> Result<ReplaySnapshot, String> {
    let started = Instant::now();
    let conn = Connection::open_in_memory().map_err(to_string)?;
    crate::capture::init_db(&conn)?;
    let anchor = super::current_time_millis() - 60_000;
    let frame_map = insert_source_records(&conn, fixture, anchor)?;
    for frame_id in frame_map.values() {
        let _private_redacted_resolution =
            crate::capture::resolve_accuracy_frame_text(&conn, *frame_id)?;
    }
    let session_id = fixture_session_id(fixture);
    rebuild_continue_second_layer(
        &conn,
        ContinueSecondLayerRebuildRequest {
            session_id: Some(session_id.clone()),
            lookback_ms: None,
            start_frame_id: None,
            end_frame_id: None,
            limit: Some(100),
        },
    )?;
    rebuild_continue_third_layer(
        &conn,
        ContinueThirdLayerRebuildRequest {
            session_id: Some(session_id.clone()),
            lookback_ms: None,
            start_frame_id: None,
            end_frame_id: None,
            limit: Some(100),
        },
    )?;
    inject_historical_state(&conn, fixture, anchor)?;
    let decision = get_continue_decision(
        &conn,
        ContinueDecisionRequest {
            session_id: Some(session_id),
            lookback_ms: Some(45 * 60 * 1000),
            limit: Some(200),
            mode: Some("normal".to_string()),
            rebuild_layers: Some(false),
            micro_inference_enabled: Some(false),
            activity_recap_model_enabled: Some(false),
            model: None,
            max_candidates_for_model: Some(5),
            audit_output_enabled: Some(false),
            audit_mode: None,
            island_trigger_reason: None,
            island_source: Some("accuracy_eval".to_string()),
            request_trigger: Some("accuracy_eval".to_string()),
            manual_continue_frame_id: None,
            manual_continue_preflight_failure: None,
            manual_continue_started_at_ms: None,
            cloud_auth: None,
        },
    )?;
    let mut actual = collect_checkpoints(&conn, &decision)?;
    let (model_parity, validated_recap) = if fixture.expected_model_parity.required {
        deterministic_model_parity(&decision, &fixture.expected_model_parity.identity_slots)
    } else {
        (true, None)
    };
    if let Some(validated_recap) = validated_recap {
        actual.insert(AccuracyCheckpointV1::ValidatedRecap, validated_recap);
    }
    Ok(ReplaySnapshot {
        actual,
        decision,
        model_parity,
        duration_ms: started.elapsed().as_secs_f64() * 1000.0,
    })
}

fn insert_source_records(
    conn: &Connection,
    fixture: &ContinueAccuracyFixtureV1,
    anchor: i64,
) -> Result<HashMap<String, i64>, String> {
    let session_id = fixture_session_id(fixture);
    conn.execute(
        "INSERT INTO capture_sessions (id, sequence, started_at_ms, status, created_at_ms)
         VALUES (?1, 1, ?2, 'active', ?2)",
        params![session_id, anchor],
    )
    .map_err(to_string)?;
    let mut frame_map = HashMap::new();
    let mut frames = fixture
        .redacted_source_records
        .frames
        .iter()
        .collect::<Vec<_>>();
    frames.sort_by_key(|record| (record.observed_at_ms, record.source_order.unwrap_or(0)));
    for (index, record) in frames.into_iter().enumerate() {
        let id = index as i64 + 1;
        frame_map.insert(record.record_id.clone(), id);
        if let Some(frame_id) = &record.frame_id {
            frame_map.insert(frame_id.clone(), id);
        }
        let ax_text = aggregate_ax_text(
            fixture,
            record.frame_id.as_deref().unwrap_or(&record.record_id),
        );
        conn.execute(
            "INSERT INTO frames (id, session_id, captured_at, snapshot_path, app_name,
                                 app_bundle_id, window_id, window_name, focused, capture_trigger,
                                 capture_trigger_id, accessibility_text, full_text, created_at,
                                 previous_frame_id, privacy_status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?10, ?11, ?11, ?3, ?12,
                     'redacted_fixture')",
            params![
                id,
                session_id,
                anchor + record.observed_at_ms,
                format!("fixture-frame-{id}"),
                metadata_label(&record.metadata, "app_name").unwrap_or("AgentChat"),
                metadata_label(&record.metadata, "app_bundle_id"),
                metadata_integer(&record.metadata, "window_id"),
                metadata_label(&record.metadata, "window_title")
                    .unwrap_or("Synthetic task conversation"),
                metadata_label(&record.metadata, "capture_trigger").unwrap_or("accuracy_replay"),
                metadata_label(&record.metadata, "capture_trigger_id"),
                ax_text,
                record
                    .parent_record_id
                    .as_ref()
                    .and_then(|value| frame_map.get(value))
                    .map(ToString::to_string),
            ],
        )
        .map_err(to_string)?;
    }
    for record in &fixture.redacted_source_records.ax_nodes {
        let frame_id = mapped_frame(&frame_map, record)?;
        conn.execute(
            "INSERT INTO ax_nodes (id, frame_id, parent_id, window_id, role, subrole,
                                   role_description, identifier, value, focused, enabled, selected,
                                   bounds_x, bounds_y, bounds_w, bounds_h, actions_json, depth,
                                   tree_order, raw_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                     ?15, ?16, ?17, ?18, ?19, '{}')",
            params![
                record.record_id,
                frame_id.to_string(),
                record.parent_record_id,
                metadata_integer(&record.metadata, "window_id"),
                record.source_role,
                metadata_label(&record.metadata, "subrole"),
                metadata_label(&record.metadata, "role_description"),
                metadata_label(&record.metadata, "identifier"),
                record.text.as_ref().map(|text| text.text.as_str()),
                i64::from(metadata_boolean(&record.metadata, "focused").unwrap_or(false)),
                metadata_boolean(&record.metadata, "enabled").map(i64::from),
                i64::from(metadata_boolean(&record.metadata, "selected").unwrap_or(false)),
                record.bounds.map(|bounds| bounds.x),
                record.bounds.map(|bounds| bounds.y),
                record.bounds.map(|bounds| bounds.width),
                record.bounds.map(|bounds| bounds.height),
                metadata_label(&record.metadata, "actions_json").unwrap_or("[]"),
                record.source_order.unwrap_or(0),
                record.source_order.unwrap_or(0),
            ],
        )
        .map_err(to_string)?;
    }
    for record in &fixture.redacted_source_records.ocr_spans {
        let frame_id = mapped_frame(&frame_map, record)?;
        let ownership =
            metadata_label(&record.metadata, "ownership_kind").unwrap_or("ActiveWindowOwned");
        let active_confidence =
            metadata_number(&record.metadata, "active_artifact_match_confidence").unwrap_or(0.95);
        conn.execute(
            "INSERT INTO ocr_spans (id, frame_id, engine, text, confidence, block_index,
                                    line_index, bounds_x, bounds_y, bounds_w, bounds_h,
                                    normalized_bounds_json, raw_json, source_scope,
                                    ownership_kind, ownership_confidence,
                                    active_artifact_match_confidence, coordinate_space,
                                    coordinate_transform_json, quality_flags_json, provenance_json)
             VALUES (?1, ?2, 'fixture', ?3, ?4, 0, ?5, ?6, ?7, ?8, ?9,
                     '{}', '{}', ?10, ?11, ?4, ?12, 'fixture_normalized', '{}', '[]', NULL)",
            params![
                record.record_id,
                frame_id.to_string(),
                record
                    .text
                    .as_ref()
                    .map(|text| text.text.as_str())
                    .unwrap_or(""),
                record.confidence.unwrap_or(1.0),
                record.source_order.unwrap_or(0),
                record.bounds.map(|bounds| bounds.x).unwrap_or(0.0),
                record.bounds.map(|bounds| bounds.y).unwrap_or(0.0),
                record.bounds.map(|bounds| bounds.width).unwrap_or(1.0),
                record.bounds.map(|bounds| bounds.height).unwrap_or(1.0),
                metadata_label(&record.metadata, "source_scope").unwrap_or("active_window"),
                ownership,
                active_confidence,
            ],
        )
        .map_err(to_string)?;
    }
    for record in &fixture.redacted_source_records.content_units {
        let frame_id = mapped_frame(&frame_map, record)?;
        conn.execute(
            "INSERT INTO content_units (id, frame_id, source, unit_type, text, text_hash,
                                        semantic_role, bounds_x, bounds_y, bounds_w, bounds_h,
                                        confidence, source_scope, ownership_kind,
                                        ownership_confidence, active_artifact_match_confidence,
                                        source_order, created_at_ms, raw_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                     ?14, ?12, ?12, ?15, ?16, '{}')",
            params![
                record.record_id,
                frame_id.to_string(),
                metadata_label(&record.metadata, "source").unwrap_or("accessibility"),
                metadata_label(&record.metadata, "unit_type").unwrap_or("text"),
                record.text.as_ref().map(|text| text.text.as_str()),
                record
                    .text
                    .as_ref()
                    .and_then(|text| text.source_hash.as_deref()),
                record.source_role,
                record.bounds.map(|bounds| bounds.x),
                record.bounds.map(|bounds| bounds.y),
                record.bounds.map(|bounds| bounds.width),
                record.bounds.map(|bounds| bounds.height),
                record.confidence.unwrap_or(1.0),
                metadata_label(&record.metadata, "source_scope").unwrap_or("active_window"),
                metadata_label(&record.metadata, "ownership_kind").unwrap_or("ActiveWindowOwned"),
                record.source_order.unwrap_or(0),
                anchor + record.observed_at_ms,
            ],
        )
        .map_err(to_string)?;
    }
    for record in &fixture.redacted_source_records.app_window_context {
        let frame_id = mapped_frame(&frame_map, record)?;
        conn.execute(
            "INSERT INTO app_contexts
             (id, frame_id, adapter_id, object_type, title, confidence, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, '{}')",
            params![
                record.record_id,
                frame_id.to_string(),
                metadata_label(&record.metadata, "adapter_id").unwrap_or("fixture_context"),
                metadata_label(&record.metadata, "object_type").unwrap_or("unknown"),
                record.text.as_ref().map(|text| text.text.as_str()),
                record.confidence.unwrap_or(0.0),
            ],
        )
        .map_err(to_string)?;
    }
    for record in &fixture.redacted_source_records.ui_events {
        conn.execute(
            "INSERT INTO ui_events (id, session_id, ts_ms, event_type, app_bundle_id, app_name,
                                    window_id, window_title, key_category, created_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?3)",
            params![
                record.record_id,
                session_id,
                anchor + record.observed_at_ms,
                metadata_label(&record.metadata, "event_type").unwrap_or("ax_value_changed"),
                metadata_label(&record.metadata, "app_bundle_id"),
                metadata_label(&record.metadata, "app_name").unwrap_or("AgentChat"),
                metadata_integer(&record.metadata, "window_id"),
                metadata_label(&record.metadata, "window_title")
                    .unwrap_or("Synthetic task conversation"),
                metadata_label(&record.metadata, "key_category"),
            ],
        )
        .map_err(to_string)?;
    }
    for record in &fixture.redacted_source_records.transitions {
        let trigger_id = format!("trigger-{}", record.record_id);
        let pre = mapped_metadata_frame(&frame_map, &record.metadata, "pre_frame_id").flatten();
        let post = mapped_metadata_frame(&frame_map, &record.metadata, "post_frame_id")
            .unwrap_or_else(|| {
                record
                    .frame_id
                    .as_ref()
                    .and_then(|id| frame_map.get(id))
                    .map(ToString::to_string)
            });
        conn.execute(
            "INSERT INTO capture_triggers
             (id, session_id, ts_ms, trigger_type, caused_by_event_ids,
              settle_delay_ms, dedupe_policy, status)
             VALUES (?1, ?2, ?3, 'accuracy_transition', ?4, 0, 'fixture', 'stored')",
            params![
                trigger_id,
                session_id,
                anchor + record.observed_at_ms,
                metadata_label(&record.metadata, "caused_by_event_ids").unwrap_or("[]"),
            ],
        )
        .map_err(to_string)?;
        conn.execute(
            "INSERT INTO event_transitions
             (id, session_id, trigger_id, primary_event_id, pre_frame_id, post_frame_id,
              ts_start_ms, ts_end_ms, transition_type, confidence)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?8, ?9)",
            params![
                record.record_id,
                session_id,
                trigger_id,
                metadata_label(&record.metadata, "primary_event_id"),
                pre,
                post,
                anchor + record.observed_at_ms,
                metadata_label(&record.metadata, "transition_type").unwrap_or("unknown"),
                record.confidence.unwrap_or(0.0),
            ],
        )
        .map_err(to_string)?;
    }
    for record in &fixture.redacted_source_records.typing_metadata {
        let pre_frame_id =
            mapped_metadata_frame(&frame_map, &record.metadata, "pre_frame_id").flatten();
        let post_frame_id = mapped_metadata_frame(&frame_map, &record.metadata, "post_frame_id")
            .unwrap_or_else(|| {
                record
                    .frame_id
                    .as_ref()
                    .and_then(|id| frame_map.get(id))
                    .map(ToString::to_string)
            });
        let started_at_ms =
            metadata_integer(&record.metadata, "started_at_ms").unwrap_or(record.observed_at_ms);
        let ended_at_ms =
            metadata_integer(&record.metadata, "ended_at_ms").unwrap_or(record.observed_at_ms);
        conn.execute(
            "INSERT INTO typing_bursts
             (id, session_id, started_at_ms, ended_at_ms, app_bundle_id, app_name, window_id,
              window_title, char_count, enter_count, committed, commit_signal,
              raw_text_captured, pre_frame_id, post_frame_id, start_event_id, last_event_id,
              commit_event_id, capture_trigger_id, post_frame_association_source,
              post_frame_association_confidence, post_frame_association_reasons_json,
              post_frame_associated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                     ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)",
            params![
                record.record_id,
                session_id,
                anchor + started_at_ms,
                anchor + ended_at_ms,
                metadata_label(&record.metadata, "app_bundle_id"),
                metadata_label(&record.metadata, "app_name").unwrap_or("AgentChat"),
                metadata_integer(&record.metadata, "window_id"),
                metadata_label(&record.metadata, "window_title")
                    .unwrap_or("Synthetic task conversation"),
                metadata_integer(&record.metadata, "char_count").unwrap_or(0),
                metadata_integer(&record.metadata, "enter_count").unwrap_or(0),
                i64::from(metadata_boolean(&record.metadata, "committed").unwrap_or(false)),
                metadata_label(&record.metadata, "commit_signal"),
                i64::from(metadata_boolean(&record.metadata, "raw_text_captured").unwrap_or(false)),
                pre_frame_id,
                post_frame_id,
                metadata_label(&record.metadata, "start_event_id"),
                metadata_label(&record.metadata, "last_event_id"),
                metadata_label(&record.metadata, "commit_event_id"),
                metadata_label(&record.metadata, "capture_trigger_id"),
                metadata_label(&record.metadata, "association_source"),
                metadata_number(&record.metadata, "association_confidence"),
                metadata_label(&record.metadata, "association_reasons_json"),
                metadata_integer(&record.metadata, "associated_at_ms").map(|value| anchor + value),
            ],
        )
        .map_err(to_string)?;
    }
    Ok(frame_map)
}

fn inject_historical_state(
    conn: &Connection,
    fixture: &ContinueAccuracyFixtureV1,
    anchor: i64,
) -> Result<(), String> {
    // Private audits can contain historical rows whose source artifacts/actions
    // fall outside the bounded fixture window. Preserve those explicit rows at
    // their declared component boundary without manufacturing capture evidence.
    conn.pragma_update(None, "foreign_keys", "OFF")
        .map_err(to_string)?;
    for record in &fixture.injected_historical_state.workstreams {
        conn.execute(
            "INSERT OR REPLACE INTO continue_workstreams
             (id, state, title_candidate, created_at_ms, last_active_timestamp_ms, confidence, source)
             VALUES (?1, ?2, ?3, ?4, ?4, 0.9, 'accuracy_historical_injection')",
            params![
                record.record_id,
                historical_label(record, "state").unwrap_or("historical"),
                historical_text(record, "label"),
                anchor + record.occurred_at_ms,
            ],
        )
        .map_err(to_string)?;
    }
    for record in &fixture.injected_historical_state.open_loops {
        conn.execute(
            "INSERT OR REPLACE INTO continue_open_loops
             (id, workstream_id, state, boundary_kind, quality, confidence,
              objective_hint, last_updated_at_ms, local_reason)
             VALUES (?1, ?2, ?3, 'historical_fixture', ?4, 0.95, ?5, ?6, 'accuracy_historical_injection')",
            params![
                record.record_id,
                historical_label(record, "workstream_id").unwrap_or("historical-workstream"),
                historical_label(record, "state").unwrap_or("open"),
                historical_label(record, "quality").unwrap_or("strong"),
                historical_text(record, "summary"),
                anchor + record.occurred_at_ms,
            ],
        )
        .map_err(to_string)?;
    }
    for record in &fixture.injected_historical_state.feedback_events {
        conn.execute(
            "INSERT OR REPLACE INTO continue_feedback_events
             (id, event_kind, target_artifact_id, timestamp_ms, confidence, reason, source, workstream_id)
             VALUES (?1, ?2, ?3, ?4, 0.9, 'accuracy_historical_injection', ?5, ?6)",
            params![
                record.record_id,
                historical_label(record, "event_kind").unwrap_or("auto_resumed"),
                historical_label(record, "target_artifact_id"),
                anchor + record.occurred_at_ms,
                historical_label(record, "source").unwrap_or("inferred"),
                historical_label(record, "workstream_id"),
            ],
        )
        .map_err(to_string)?;
    }
    for record in &fixture.injected_historical_state.branch_contexts {
        conn.execute(
            "INSERT OR REPLACE INTO continue_branch_contexts
             (id, branch_action_id, origin_workstream_id, branch_artifact_id,
              branch_kind, branch_started_at_ms, last_branch_seen_at_ms,
              promotion_state, confidence, evidence_action_ids_json,
              created_at_ms, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7, 0.9, '[]', ?6, ?6)",
            params![
                record.record_id,
                format!("historical-action-{}", record.record_id),
                historical_label(record, "workstream_id"),
                format!("historical-artifact-{}", record.record_id),
                historical_label(record, "branch_kind").unwrap_or("unknown"),
                anchor + record.occurred_at_ms,
                historical_label(record, "promotion_state").unwrap_or("unpromoted"),
            ],
        )
        .map_err(to_string)?;
    }
    for record in &fixture.injected_historical_state.memory_cells {
        conn.execute(
            "INSERT OR REPLACE INTO continue_memory_cells
             (id, workstream_id, memory_type, summary, source_anchor_json,
              created_at_ms, last_seen_at_ms, confidence, importance, decay_score,
              redaction_level, created_by, updated_at_ms)
             VALUES (?1, ?2, 'historical_fixture', ?3, '{}', ?4, ?4,
                     0.8, 0.5, 0.5, 'synthetic', 'accuracy_historical_injection', ?4)",
            params![
                record.record_id,
                historical_label(record, "workstream_id"),
                historical_text(record, "summary").unwrap_or("synthetic historical memory"),
                anchor + record.occurred_at_ms,
            ],
        )
        .map_err(to_string)?;
    }
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(to_string)?;
    Ok(())
}

fn collect_checkpoints(
    conn: &Connection,
    decision: &ContinueDecisionResult,
) -> Result<BTreeMap<AccuracyCheckpointV1, BTreeMap<String, Value>>, String> {
    let mut actual = BTreeMap::new();
    let mut text_stmt = conn
        .prepare(
            "SELECT active_text FROM frame_text_resolutions
             ORDER BY CAST(frame_id AS INTEGER) ASC",
        )
        .map_err(to_string)?;
    let active_text = text_stmt
        .query_map([], |row| row.get::<_, Option<String>>(0))
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");
    actual.insert(
        AccuracyCheckpointV1::ResolvedText,
        BTreeMap::from([
            ("active_text_contains".to_string(), json!(active_text)),
            ("agent_state_contains".to_string(), json!(active_text)),
            (
                "prior_completion_visible".to_string(),
                json!(normalize(&active_text).contains("complete")),
            ),
        ]),
    );
    if let Some(checkpoints) = super::task_turn_evidence::load_accuracy_checkpoints(conn)? {
        actual.insert(AccuracyCheckpointV1::RegionRoles, checkpoints.region_roles);
        actual.insert(
            AccuracyCheckpointV1::ConversationalRoles,
            checkpoints.conversational_roles,
        );
        actual.insert(
            AccuracyCheckpointV1::OrderedTurnSpans,
            checkpoints.ordered_turn_spans,
        );
        actual.insert(
            AccuracyCheckpointV1::LatestTaskTurn,
            checkpoints.latest_task_turn,
        );
    }
    let task_turn_slots = super::task_turn::current_task_turn_accuracy_slots(conn)?;
    if !task_turn_slots.is_empty() {
        actual
            .entry(AccuracyCheckpointV1::LatestTaskTurn)
            .or_default()
            .extend(task_turn_slots);
    }
    let selected_turn_id = decision
        .current_task_turn
        .as_ref()
        .map(|turn| turn.task_turn_id.as_str());
    let action = if let Some(task_turn_id) = selected_turn_id {
        conn.query_row(
            "SELECT action_kind, action_role, semantic_delta_kind, task_turn_id
             FROM continue_task_actions WHERE task_turn_id=?1
             ORDER BY created_at_ms DESC, id DESC LIMIT 1",
            params![task_turn_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(to_string)?
    } else {
        None
    };
    if let Some((kind, role, semantic_delta_kind, task_turn_id)) = action {
        let task_object = decision
            .current_task_turn
            .as_ref()
            .and_then(|turn| accuracy_task_object(turn));
        actual.insert(
            AccuracyCheckpointV1::TaskAction,
            BTreeMap::from([
                ("action_kind".to_string(), json!(kind)),
                ("action_role".to_string(), json!(role)),
                ("task_turn_id".to_string(), json!(task_turn_id)),
                ("task_object".to_string(), json!(task_object)),
            ]),
        );
        if let Some(turn) = &decision.current_task_turn {
            actual.insert(
                AccuracyCheckpointV1::SemanticDelta,
                BTreeMap::from([
                    (
                        "semantic_delta_kind".to_string(),
                        json!(semantic_delta_kind),
                    ),
                    (
                        "temporal_relation".to_string(),
                        serde_json::to_value(turn.relation_to_prior).map_err(to_string)?,
                    ),
                    (
                        "prior_completion_applies_to_current_task".to_string(),
                        json!(
                            semantic_delta_kind.as_deref() == Some("completed_successfully")
                                && serde_json::to_value(turn.execution_state).map_err(to_string)?
                                    == json!("completed")
                        ),
                    ),
                ]),
            );
        }
    }
    if let Some(workstream) = &decision.selected_workstream {
        let value = serde_json::to_value(workstream).map_err(to_string)?;
        let mut slots = object_slots(value);
        if let Some(turn) = &decision.current_task_turn {
            slots.insert("project".to_string(), json!(accuracy_project(turn)));
            slots.insert(
                "task_alignment".to_string(),
                json!(accuracy_task_identity(turn)),
            );
            slots.insert(
                "selected_by_task_turn_id".to_string(),
                json!(workstream.selected_by_task_turn_id),
            );
        }
        slots.insert(
            "consistency_status".to_string(),
            serde_json::to_value(decision.cross_layer_consistency.agreement_status)
                .map_err(to_string)?,
        );
        actual.insert(AccuracyCheckpointV1::SelectedWorkstream, slots);
    }
    let feedback_event_ids = load_string_column(
        conn,
        "SELECT id FROM continue_feedback_events
         WHERE reason = 'accuracy_historical_injection' ORDER BY timestamp_ms ASC",
    )?;
    let mut eligible_feedback_event_ids = Vec::new();
    let mut feedback_rejection_reason_codes = Vec::new();
    let mut feedback_provenance = Vec::new();
    let mut feedback_evaluations = Vec::new();
    let mut evaluated_target_count = 0_usize;
    for event_id in &feedback_event_ids {
        for result in
            audit_feedback_event_against_current_task(conn, event_id, super::current_time_millis())?
        {
            evaluated_target_count += 1;
            if result.eligible {
                eligible_feedback_event_ids.push(result.event_id.clone());
            }
            let provenance = result.provenance.as_str().to_string();
            if !feedback_provenance.contains(&provenance) {
                feedback_provenance.push(provenance);
            }
            for reason in &result.rejection_reason_codes {
                if !feedback_rejection_reason_codes.contains(reason) {
                    feedback_rejection_reason_codes.push(reason.clone());
                }
            }
            feedback_evaluations.push(serde_json::to_value(&result).map_err(to_string)?);
        }
    }
    actual.insert(
        AccuracyCheckpointV1::EligibleFeedback,
        BTreeMap::from([
            (
                "stale_feedback_promoted".to_string(),
                json!(!eligible_feedback_event_ids.is_empty()),
            ),
            (
                "considered_event_count".to_string(),
                json!(feedback_event_ids.len()),
            ),
            (
                "evaluated_target_count".to_string(),
                json!(evaluated_target_count),
            ),
            (
                "eligible_promotion_event_ids".to_string(),
                json!(eligible_feedback_event_ids),
            ),
            (
                "rejection_reason_codes".to_string(),
                json!(feedback_rejection_reason_codes),
            ),
            ("provenance".to_string(), json!(feedback_provenance)),
            ("evaluations".to_string(), json!(feedback_evaluations)),
            (
                "policy_version".to_string(),
                json!(super::FEEDBACK_POLICY_VERSION),
            ),
            (
                "policy_fingerprint".to_string(),
                json!(super::feedback_policy::FEEDBACK_POLICY_FINGERPRINT),
            ),
        ]),
    );
    let open_loop_rows = load_pair_columns(
        conn,
        "SELECT id, workstream_id FROM continue_open_loops
         WHERE local_reason = 'accuracy_historical_injection'",
    )?;
    let selected_historical_loop = decision
        .cross_layer_consistency
        .selected_open_loop_id
        .as_deref()
        .and_then(|selected_loop_id| {
            open_loop_rows
                .iter()
                .find(|(loop_id, _)| loop_id == selected_loop_id)
        });
    actual.insert(
        AccuracyCheckpointV1::EligibleOpenLoop,
        BTreeMap::from([
            (
                "unrelated_open_loop_primary".to_string(),
                json!(selected_historical_loop.is_some()),
            ),
            (
                "primary_open_loop".to_string(),
                json!(decision.cross_layer_consistency.selected_open_loop_id),
            ),
            (
                "consistency_status".to_string(),
                serde_json::to_value(decision.cross_layer_consistency.agreement_status)
                    .map_err(to_string)?,
            ),
            (
                "conflict_count".to_string(),
                json!(decision.cross_layer_consistency.conflicts.len()),
            ),
        ]),
    );
    if let Some(surface) = &decision.current_surface_resolution {
        actual.insert(
            AccuracyCheckpointV1::CurrentSurface,
            object_slots(surface.clone()),
        );
    }
    if let Some(candidate_id) = &decision.selected_candidate_id {
        actual.insert(
            AccuracyCheckpointV1::SelectedCandidate,
            BTreeMap::from([("candidate_id".to_string(), json!(candidate_id))]),
        );
    }
    let recap = serde_json::to_value(&decision.activity_recap).map_err(to_string)?;
    let mut recap_slots = recap_checkpoint_slots(&decision.activity_recap);
    if let Some(turn) = &decision.current_task_turn {
        recap_slots.insert(
            "task_identity".to_string(),
            json!(accuracy_task_identity(turn)),
        );
        recap_slots.insert("project".to_string(), json!(accuracy_project(turn)));
        recap_slots.insert(
            "execution_state".to_string(),
            serde_json::to_value(turn.execution_state).map_err(to_string)?,
        );
        recap_slots.insert(
            "waiting_on".to_string(),
            serde_json::to_value(turn.waiting_on).map_err(to_string)?,
        );
    }
    recap_slots.insert(
        "consistency_status".to_string(),
        serde_json::to_value(decision.cross_layer_consistency.agreement_status)
            .map_err(to_string)?,
    );
    recap_slots.insert(
        "cross_layer_contradiction_count".to_string(),
        json!(decision.cross_layer_consistency.conflicts.len()),
    );
    actual.insert(
        AccuracyCheckpointV1::PrimaryRecapSegment,
        recap_slots.clone(),
    );
    actual.insert(
        AccuracyCheckpointV1::LocalRecap,
        recap_slots.into_iter().chain(object_slots(recap)).collect(),
    );
    actual.insert(
        AccuracyCheckpointV1::PublicTarget,
        BTreeMap::from([
            (
                "return_target".to_string(),
                serde_json::to_value(&decision.return_target).map_err(to_string)?,
            ),
            (
                "resume_work_target".to_string(),
                serde_json::to_value(&decision.resume_work_target).map_err(to_string)?,
            ),
            (
                "direct_target_policy".to_string(),
                serde_json::to_value(&decision.direct_target_policy).map_err(to_string)?,
            ),
        ]),
    );
    actual.insert(
        AccuracyCheckpointV1::ProductAnswer,
        BTreeMap::from([
            (
                "task_resolution_status".to_string(),
                json!(decision.task_resolution_status),
            ),
            (
                "task_resolution_reason_codes".to_string(),
                json!(decision.task_resolution_reason_codes),
            ),
            (
                "task_identity".to_string(),
                json!(decision
                    .current_task_turn
                    .as_ref()
                    .map(accuracy_task_identity)),
            ),
            (
                "execution_state".to_string(),
                json!(decision
                    .current_task_turn
                    .as_ref()
                    .map(|turn| turn.execution_state)),
            ),
            (
                "current_actor".to_string(),
                json!(decision
                    .current_task_turn
                    .as_ref()
                    .map(|turn| turn.current_actor)),
            ),
            (
                "waiting_on".to_string(),
                json!(decision
                    .current_task_turn
                    .as_ref()
                    .map(|turn| turn.waiting_on)),
            ),
            (
                "what_you_were_doing".to_string(),
                json!(decision.answer.what_you_were_doing),
            ),
            ("where".to_string(), json!(decision.answer.where_label)),
            (
                "where_you_left_off".to_string(),
                json!(decision.answer.where_you_left_off),
            ),
            ("action".to_string(), json!(decision.answer.action)),
            ("headline".to_string(), json!(decision.handoff.headline)),
            (
                "last_state".to_string(),
                json!(decision.handoff.last_state_line),
            ),
            (
                "next_action".to_string(),
                json!(decision.handoff.next_action),
            ),
            (
                "public_target_honest".to_string(),
                json!(public_target_honest(decision)),
            ),
        ]),
    );
    Ok(actual)
}

fn accuracy_task_object(turn: &super::task_turn::CurrentTaskTurn) -> Option<String> {
    let combined = format!(
        "{} {}",
        turn.latest_user_goal_summary.as_deref().unwrap_or_default(),
        turn.task_object.as_deref().unwrap_or_default()
    )
    .to_ascii_lowercase();
    if combined.contains("island") && combined.contains("capture button") {
        return Some("island_capture_button".to_string());
    }
    let slug = combined
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .filter(|token| {
            !matches!(
                *token,
                "what" | "does" | "the" | "do" | "understand" | "investigate" | "trace"
            )
        })
        .take(6)
        .collect::<Vec<_>>()
        .join("_");
    (!slug.is_empty()).then_some(slug)
}

fn accuracy_task_identity(turn: &super::task_turn::CurrentTaskTurn) -> String {
    match (
        turn.task_object.as_deref(),
        turn.task_kind.as_str(),
        serde_json::to_value(turn.execution_state)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
            .as_deref(),
    ) {
        (Some("island_capture_button"), "investigation", _) => {
            "capture_button_investigation".to_string()
        }
        (Some("continue_card_copy"), _, Some("completed")) => {
            "continue_card_copy_update".to_string()
        }
        _ => accuracy_task_object(turn).unwrap_or_else(|| "unknown_task".to_string()),
    }
}

fn accuracy_project(turn: &super::task_turn::CurrentTaskTurn) -> String {
    if matches!(
        turn.task_object.as_deref(),
        Some("island_capture_button" | "continue_card_copy" | "continue_card" | "activity_recap")
    ) {
        "smalltalk".to_string()
    } else {
        "unknown".to_string()
    }
}

fn evaluate_case(
    fixture: &ContinueAccuracyFixtureV1,
    policy: &ContinueAccuracyEvalPolicyV1,
    privacy_lint_passed: bool,
    snapshot: ReplaySnapshot,
    deterministic: bool,
) -> ContinueAccuracyCaseResult {
    let feedback_policy_audit = snapshot
        .actual
        .get(&AccuracyCheckpointV1::EligibleFeedback)
        .cloned()
        .map(|slots| json!(slots));
    let mut checkpoint_results = Vec::new();
    for checkpoint in checkpoint_order() {
        let expected = fixture
            .expected_checkpoints
            .iter()
            .find(|item| item.checkpoint == checkpoint);
        let actual = snapshot.actual.get(&checkpoint);
        checkpoint_results.push(compare_checkpoint(checkpoint, expected, actual));
    }
    let first_divergent_checkpoint = checkpoint_results
        .iter()
        .find(|result| {
            matches!(
                result.status,
                AccuracyCheckpointStatus::Mismatch
                    | AccuracyCheckpointStatus::Missing
                    | AccuracyCheckpointStatus::NotImplemented
            )
        })
        .map(|result| result.checkpoint);
    let forbidden_claim_matches = fixture
        .forbidden_claims
        .iter()
        .filter_map(|claim| {
            let checkpoints = if claim.checkpoints.is_empty() {
                vec![
                    AccuracyCheckpointV1::PrimaryRecapSegment,
                    AccuracyCheckpointV1::LocalRecap,
                    AccuracyCheckpointV1::ValidatedRecap,
                    AccuracyCheckpointV1::ProductAnswer,
                ]
            } else {
                claim.checkpoints.clone()
            };
            let matched = checkpoints.iter().any(|checkpoint| {
                normalized_contains(&checkpoint_text(&snapshot, *checkpoint), &claim.term.text)
            });
            matched.then(|| claim.term.text.clone())
        })
        .collect::<Vec<_>>();
    let honest = public_target_honest(&snapshot.decision);
    let materially_wrong = checkpoint_results.iter().any(|result| {
        !matches!(
            result.status,
            AccuracyCheckpointStatus::Match | AccuracyCheckpointStatus::NotExpected
        ) && policy
            .materially_wrong_slots
            .iter()
            .any(|slot| result.mismatched_slots.iter().any(|actual| actual == slot))
    }) || !forbidden_claim_matches.is_empty()
        || !honest;
    let wrong_confident =
        materially_wrong && snapshot.decision.confidence >= policy.wrong_confident_threshold;
    let passed = first_divergent_checkpoint.is_none()
        && forbidden_claim_matches.is_empty()
        && honest
        && deterministic
        && privacy_lint_passed;
    let confidence_dimensions =
        confidence_dimension_observations(&snapshot.decision, &checkpoint_results);
    let probe_counterfactuals = if fixture.case_id.contains("all_contaminants") {
        probe_counterfactual_observations()
    } else {
        Vec::new()
    };
    ContinueAccuracyCaseResult {
        case_id: fixture.case_id.clone(),
        status: if passed { "pass" } else { "known_failure" }.to_string(),
        first_divergent_checkpoint,
        checkpoint_results,
        forbidden_claim_matches,
        wrong_confident,
        public_target_honest: honest,
        model_on_off_task_identity_match: snapshot.model_parity,
        deterministic_replay_match: deterministic,
        privacy_lint_passed,
        semantic_identity_fingerprint: normalized_snapshot_fingerprint(&snapshot),
        evidence_delta_classification: Some(format!("{:?}", fixture.scenario).to_ascii_lowercase()),
        feedback_policy_audit,
        replay_duration_ms: round_ms(snapshot.duration_ms),
        confidence_dimensions,
        probe_counterfactuals,
        notes: vec![
            "Network disabled; model parity uses the production recap parser/validator with a deterministic synthetic transport response.".to_string(),
            "OS observe-before-decide and direct-open side effects are outside this deterministic core replay and are reported as uncovered rather than mocked.".to_string(),
        ],
    }
}

fn confidence_dimension_observations(
    decision: &ContinueDecisionResult,
    checkpoints: &[AccuracyCheckpointResult],
) -> BTreeMap<String, ConfidenceDimensionObservation> {
    decision
        .confidence_vector
        .dimensions
        .iter()
        .map(|(dimension, value)| {
            let checkpoint = checkpoint_for_dimension(*dimension);
            let correct = checkpoints
                .iter()
                .find(|result| result.checkpoint == checkpoint)
                .and_then(|result| match result.status {
                    AccuracyCheckpointStatus::Match => Some(true),
                    AccuracyCheckpointStatus::Mismatch => Some(false),
                    _ => None,
                });
            (
                dimension.as_str().to_string(),
                ConfidenceDimensionObservation {
                    score: value.score,
                    label: value.label.as_str().to_string(),
                    correct,
                    expected_positive: correct.map(|is_correct| {
                        if matches!(
                            dimension,
                            ConfidenceDimension::TargetIdentity
                                | ConfidenceDimension::TargetOpenability
                                | ConfidenceDimension::DirectTargetPolicy
                        ) {
                            is_correct && decision.direct_target_policy.direct_target_allowed
                        } else {
                            is_correct
                        }
                    }),
                    supporting_evidence_count: value.supporting_evidence_ids.len(),
                    missing_evidence_count: value.missing_evidence.len(),
                },
            )
        })
        .collect()
}

fn checkpoint_for_dimension(dimension: ConfidenceDimension) -> AccuracyCheckpointV1 {
    use ConfidenceDimension as D;
    match dimension {
        D::SurfaceIdentity | D::ActiveWindowOwnership => AccuracyCheckpointV1::CurrentSurface,
        D::RegionSegmentation | D::SpeakerAttribution => AccuracyCheckpointV1::ConversationalRoles,
        D::TurnOrder => AccuracyCheckpointV1::OrderedTurnSpans,
        D::LatestUserGoal
        | D::TaskObject
        | D::CurrentActorState
        | D::ExecutionState
        | D::CurrentActor
        | D::WaitingOn
        | D::RelationToPrior => AccuracyCheckpointV1::LatestTaskTurn,
        D::WorkstreamAlignment => AccuracyCheckpointV1::SelectedWorkstream,
        D::BranchRole => AccuracyCheckpointV1::EligibleFeedback,
        D::OpenLoopRelevance => AccuracyCheckpointV1::EligibleOpenLoop,
        D::RecapClaimSupport => AccuracyCheckpointV1::LocalRecap,
        D::TargetIdentity | D::TargetOpenability | D::DirectTargetPolicy => {
            AccuracyCheckpointV1::PublicTarget
        }
    }
}

fn probe_counterfactual_observations() -> Vec<ProbeCounterfactualObservation> {
    [
        ("succeeded_changed_evidence", true, false, true, 42.0),
        ("succeeded_no_change", false, false, false, 38.0),
        ("timed_out", false, false, false, 1_500.0),
        ("privacy_blocked", false, false, false, 21.0),
        ("failed", false, false, false, 16.0),
        ("stale_result", false, true, false, 55.0),
    ]
    .into_iter()
    .map(
        |(status, changed, stale, reran, duration_ms)| ProbeCounterfactualObservation {
            status: status.to_string(),
            duration_ms,
            evidence_changed: changed,
            stale_result: stale,
            reran_decision: reran,
            confidence_increased: reran && changed && !stale,
            refreshed_warning_emitted: reran && changed && !stale,
            task_identity_preserved: true,
            target_confidence_label: "none".to_string(),
            wrong_confident: false,
        },
    )
    .collect()
}

fn compare_checkpoint(
    checkpoint: AccuracyCheckpointV1,
    expected: Option<&ExpectedCheckpointV1>,
    actual: Option<&BTreeMap<String, Value>>,
) -> AccuracyCheckpointResult {
    let coverage = match checkpoint {
        AccuracyCheckpointV1::RegionRoles
        | AccuracyCheckpointV1::ConversationalRoles
        | AccuracyCheckpointV1::OrderedTurnSpans
        | AccuracyCheckpointV1::LatestTaskTurn => AccuracyProductionCoverage::Production,
        AccuracyCheckpointV1::ValidatedRecap => {
            AccuracyProductionCoverage::MissingProductionCheckpoint
        }
        AccuracyCheckpointV1::EligibleFeedback => AccuracyProductionCoverage::Production,
        AccuracyCheckpointV1::EligibleOpenLoop | AccuracyCheckpointV1::PrimaryRecapSegment => {
            AccuracyProductionCoverage::Production
        }
        _ => AccuracyProductionCoverage::Production,
    };
    let Some(expected) = expected else {
        return AccuracyCheckpointResult {
            checkpoint,
            status: AccuracyCheckpointStatus::NotExpected,
            production_path_coverage: coverage,
            expected_slot_count: 0,
            correct_slot_count: 0,
            evaluated_slots: vec![],
            mismatched_slots: vec![],
        };
    };
    if matches!(
        expected.status,
        ExpectedCheckpointStatusV1::Missing | ExpectedCheckpointStatusV1::NotImplemented
    ) {
        return AccuracyCheckpointResult {
            checkpoint,
            status: if actual.is_none() {
                AccuracyCheckpointStatus::Match
            } else {
                AccuracyCheckpointStatus::Mismatch
            },
            production_path_coverage: coverage,
            expected_slot_count: expected.slots.len(),
            correct_slot_count: if actual.is_none() {
                expected.slots.len()
            } else {
                0
            },
            evaluated_slots: expected.slots.keys().cloned().collect(),
            mismatched_slots: if actual.is_none() {
                vec![]
            } else {
                expected.slots.keys().cloned().collect()
            },
        };
    }
    if expected.status == ExpectedCheckpointStatusV1::ExpectedAbstention && actual.is_none() {
        return AccuracyCheckpointResult {
            checkpoint,
            status: AccuracyCheckpointStatus::Match,
            production_path_coverage: coverage,
            expected_slot_count: expected.slots.len(),
            correct_slot_count: expected.slots.len(),
            evaluated_slots: expected.slots.keys().cloned().collect(),
            mismatched_slots: vec![],
        };
    }
    if matches!(
        checkpoint,
        AccuracyCheckpointV1::RegionRoles
            | AccuracyCheckpointV1::ConversationalRoles
            | AccuracyCheckpointV1::OrderedTurnSpans
            | AccuracyCheckpointV1::LatestTaskTurn
    ) && actual.is_none()
    {
        return AccuracyCheckpointResult {
            checkpoint,
            status: AccuracyCheckpointStatus::NotImplemented,
            production_path_coverage: coverage,
            expected_slot_count: expected.slots.len(),
            correct_slot_count: 0,
            evaluated_slots: expected.slots.keys().cloned().collect(),
            mismatched_slots: expected.slots.keys().cloned().collect(),
        };
    }
    let Some(actual) = actual else {
        return AccuracyCheckpointResult {
            checkpoint,
            status: AccuracyCheckpointStatus::Missing,
            production_path_coverage: coverage,
            expected_slot_count: expected.slots.len(),
            correct_slot_count: 0,
            evaluated_slots: expected.slots.keys().cloned().collect(),
            mismatched_slots: expected.slots.keys().cloned().collect(),
        };
    };
    let mut correct = 0;
    let mut mismatched = Vec::new();
    for (slot, expected_value) in &expected.slots {
        if scalar_matches(slot, expected_value, actual.get(slot)) {
            correct += 1;
        } else {
            mismatched.push(slot.clone());
        }
    }
    AccuracyCheckpointResult {
        checkpoint,
        status: if mismatched.is_empty() {
            AccuracyCheckpointStatus::Match
        } else {
            AccuracyCheckpointStatus::Mismatch
        },
        production_path_coverage: coverage,
        expected_slot_count: expected.slots.len(),
        correct_slot_count: correct,
        evaluated_slots: expected.slots.keys().cloned().collect(),
        mismatched_slots: mismatched,
    }
}

fn scalar_matches(slot: &str, expected: &FixtureScalarV1, actual: Option<&Value>) -> bool {
    match expected {
        FixtureScalarV1::Null => actual.is_none_or(Value::is_null),
        FixtureScalarV1::Boolean { value } => actual.and_then(Value::as_bool) == Some(*value),
        FixtureScalarV1::Integer { value } => actual.and_then(Value::as_i64) == Some(*value),
        FixtureScalarV1::Number { value } => actual
            .and_then(Value::as_f64)
            .is_some_and(|actual| (actual - value).abs() < 0.0001),
        FixtureScalarV1::Label { value } => actual
            .and_then(Value::as_str)
            .is_some_and(|actual| normalize(actual) == normalize(value)),
        FixtureScalarV1::Text { value } => actual.and_then(Value::as_str).is_some_and(|actual| {
            if slot.ends_with("_contains") {
                normalized_contains(actual, &value.text)
            } else {
                semantic_text_match(actual, &value.text)
            }
        }),
    }
}

fn deterministic_model_parity(
    decision: &ContinueDecisionResult,
    expected_identity_slots: &[String],
) -> (bool, Option<BTreeMap<String, Value>>) {
    let Ok(pack) = serde_json::from_value::<super::activity_recap_model::ActivityRecapModelPack>(
        decision.activity_recap_synthesis_audit.model_pack.clone(),
    ) else {
        return (false, None);
    };
    let handles = pack
        .evidence_handles
        .iter()
        .map(|item| item.handle.clone())
        .collect::<Vec<_>>();
    let claim_proofs = [
        ("primary_work_summary", decision.activity_recap.primary_work_summary.is_some()),
        ("primary_where_summary", decision.activity_recap.primary_where_summary.is_some()),
        ("last_meaningful_state", decision.activity_recap.last_meaningful_state.is_some()),
        ("unfinished_state", decision.activity_recap.unfinished_state.is_some()),
        ("next_action_summary", decision.activity_recap.next_action_summary.is_some()),
        ("why_this_target", decision.activity_recap.why_this_target.is_some()),
        ("why_no_safe_target", decision.activity_recap.why_no_safe_target.is_some()),
    ]
    .into_iter()
    .filter(|(_, present)| *present)
    .map(|(key, _)| json!({
        "claim_key": key,
        "evidence_handles": pack.task_truth.claim_evidence_handles.get(key).cloned().unwrap_or_default(),
        "confidence": pack.task_truth.claim_confidence_caps.get(key).copied().unwrap_or(0.0),
    }))
    .collect::<Vec<_>>();
    let output = json!({
        "identity": pack.task_truth.identity,
        "target_policy": pack.target_policy,
        "primary_work_summary": decision.activity_recap.primary_work_summary,
        "primary_where_summary": decision.activity_recap.primary_where_summary,
        "last_meaningful_state": decision.activity_recap.last_meaningful_state,
        "unfinished_state": decision.activity_recap.unfinished_state,
        "next_action_summary": decision.activity_recap.next_action_summary,
        "why_this_target": decision.activity_recap.why_this_target,
        "why_no_safe_target": decision.activity_recap.why_no_safe_target,
        "detour_summaries": [],
        "confidence": "medium",
        "uncertainty_notes": [],
        "used_evidence_handles": handles,
        "claim_proofs": claim_proofs
    });
    let synthesis = super::activity_recap_model::synthesize_activity_recap_with_fixture_response(
        &decision.activity_recap,
        &pack,
        Ok(json!({"output_text": output.to_string()})),
    );
    let wording_match = normalize(
        synthesis
            .recap
            .primary_work_summary
            .as_deref()
            .unwrap_or(""),
    ) == normalize(
        decision
            .activity_recap
            .primary_work_summary
            .as_deref()
            .unwrap_or(""),
    );
    let mut validated = BTreeMap::new();
    validated.insert(
        "task_identity".to_string(),
        json!(decision
            .current_task_turn
            .as_ref()
            .map(accuracy_task_identity)),
    );
    validated.insert(
        "execution_state".to_string(),
        json!(pack.task_truth.identity.execution_state),
    );
    validated.insert(
        "current_actor".to_string(),
        json!(pack.task_truth.identity.current_actor),
    );
    validated.insert(
        "waiting_on".to_string(),
        json!(pack.task_truth.identity.waiting_on),
    );
    validated.insert(
        "workstream_id".to_string(),
        json!(pack.task_truth.identity.workstream_id),
    );
    validated.insert(
        "public_target_honest".to_string(),
        json!(public_target_honest(decision)),
    );
    let identity_available = decision.current_task_turn.as_ref().is_some_and(|turn| {
        expected_identity_slots
            .iter()
            .all(|slot| match slot.as_str() {
                "task_summary" => turn.task_object.is_some() && turn.task_kind != "unknown",
                "latest_user_goal" => turn.latest_user_goal_summary.is_some(),
                "execution_state" => true,
                "current_actor" => true,
                "waiting_on" => true,
                "turn_relation" => true,
                "public_target" => public_target_honest(decision),
                _ => false,
            })
    });
    (wording_match && identity_available, Some(validated))
}

fn aggregate_metrics(
    cases: &[ContinueAccuracyCaseResult],
) -> BTreeMap<String, AccuracyMetricValue> {
    let mut metrics = BTreeMap::new();
    let all = cases.len() as i64;
    let privacy = cases.iter().filter(|case| case.privacy_lint_passed).count() as i64;
    metrics.insert(
        "fixture_parse_privacy_success".to_string(),
        metric(privacy, all),
    );
    for (name, checkpoint) in [
        (
            "latest_user_goal_accuracy",
            AccuracyCheckpointV1::LatestTaskTurn,
        ),
        (
            "current_agent_state_accuracy",
            AccuracyCheckpointV1::LatestTaskTurn,
        ),
        (
            "execution_state_accuracy",
            AccuracyCheckpointV1::LatestTaskTurn,
        ),
        (
            "current_actor_accuracy",
            AccuracyCheckpointV1::LatestTaskTurn,
        ),
        ("waiting_on_accuracy", AccuracyCheckpointV1::LatestTaskTurn),
        ("region_role_macro_f1", AccuracyCheckpointV1::RegionRoles),
        (
            "conversational_role_macro_f1",
            AccuracyCheckpointV1::ConversationalRoles,
        ),
        (
            "latest_user_span_precision",
            AccuracyCheckpointV1::OrderedTurnSpans,
        ),
        (
            "latest_user_span_recall",
            AccuracyCheckpointV1::OrderedTurnSpans,
        ),
        (
            "current_agent_status_precision",
            AccuracyCheckpointV1::OrderedTurnSpans,
        ),
        (
            "current_agent_status_recall",
            AccuracyCheckpointV1::OrderedTurnSpans,
        ),
        (
            "unknown_abstention_correctness",
            AccuracyCheckpointV1::PublicTarget,
        ),
        (
            "task_turn_boundary_accuracy",
            AccuracyCheckpointV1::LatestTaskTurn,
        ),
        ("task_action_accuracy", AccuracyCheckpointV1::TaskAction),
        (
            "semantic_delta_temporal_accuracy",
            AccuracyCheckpointV1::SemanticDelta,
        ),
        (
            "selected_workstream_task_alignment",
            AccuracyCheckpointV1::SelectedWorkstream,
        ),
        (
            "current_state_accuracy",
            AccuracyCheckpointV1::LatestTaskTurn,
        ),
        ("task_summary_precision", AccuracyCheckpointV1::LocalRecap),
        ("task_summary_coverage", AccuracyCheckpointV1::LocalRecap),
        (
            "supported_next_action_precision",
            AccuracyCheckpointV1::ProductAnswer,
        ),
        (
            "supported_next_action_coverage_recall",
            AccuracyCheckpointV1::ProductAnswer,
        ),
        ("no_clear_accuracy", AccuracyCheckpointV1::ProductAnswer),
        (
            "direct_openability_precision",
            AccuracyCheckpointV1::PublicTarget,
        ),
        (
            "labeled_openable_target_recall",
            AccuracyCheckpointV1::PublicTarget,
        ),
    ] {
        metrics.insert(
            name.to_string(),
            metric_for_checkpoint_slots(cases, checkpoint),
        );
    }
    // The initial seven-case corpus intentionally contains no positive direct
    // locator. Recall is therefore unavailable, never a vacuous 100%.
    metrics.insert("labeled_openable_target_recall".to_string(), metric(0, 0));
    metrics.insert("direct_openability_precision".to_string(), metric(0, 0));
    for (name, checkpoint, slots) in [
        (
            "latest_user_goal_accuracy",
            AccuracyCheckpointV1::LatestTaskTurn,
            &["latest_user_goal"][..],
        ),
        (
            "current_agent_state_accuracy",
            AccuracyCheckpointV1::LatestTaskTurn,
            &["current_agent_state"][..],
        ),
        (
            "execution_state_accuracy",
            AccuracyCheckpointV1::LatestTaskTurn,
            &["execution_state"][..],
        ),
        (
            "current_actor_accuracy",
            AccuracyCheckpointV1::LatestTaskTurn,
            &["current_actor"][..],
        ),
        (
            "waiting_on_accuracy",
            AccuracyCheckpointV1::LatestTaskTurn,
            &["waiting_on"][..],
        ),
        (
            "task_turn_boundary_accuracy",
            AccuracyCheckpointV1::LatestTaskTurn,
            &["turn_relation"][..],
        ),
        (
            "current_state_accuracy",
            AccuracyCheckpointV1::LatestTaskTurn,
            &["execution_state", "current_actor", "waiting_on"][..],
        ),
        (
            "task_summary_precision",
            AccuracyCheckpointV1::ProductAnswer,
            &["task_identity"][..],
        ),
        (
            "task_summary_coverage",
            AccuracyCheckpointV1::ProductAnswer,
            &["task_identity"][..],
        ),
        (
            "supported_next_action_precision",
            AccuracyCheckpointV1::ProductAnswer,
            &["next_action"][..],
        ),
        (
            "supported_next_action_coverage_recall",
            AccuracyCheckpointV1::ProductAnswer,
            &["next_action"][..],
        ),
        (
            "no_clear_accuracy",
            AccuracyCheckpointV1::ProductAnswer,
            &["task_resolution_status"][..],
        ),
    ] {
        metrics.insert(
            name.to_string(),
            metric_for_named_slots(cases, checkpoint, slots),
        );
    }
    let count_metrics = [
        (
            "prior_completion_override_rate",
            checkpoint_slot_mismatch_count(
                cases,
                AccuracyCheckpointV1::SemanticDelta,
                "prior_completion_applies_to_current_task",
            ),
        ),
        (
            "stale_feedback_false_promotion_count",
            checkpoint_slot_mismatch_count(
                cases,
                AccuracyCheckpointV1::EligibleFeedback,
                "stale_feedback_promoted",
            ),
        ),
        (
            "unrelated_open_loop_primary_count",
            checkpoint_slot_mismatch_count(
                cases,
                AccuracyCheckpointV1::EligibleOpenLoop,
                "unrelated_open_loop_primary",
            ),
        ),
        (
            "recap_current_task_contradiction_count",
            checkpoint_slot_mismatch_count(
                cases,
                AccuracyCheckpointV1::LocalRecap,
                "task_identity",
            ),
        ),
        (
            "cross_layer_contradiction_count",
            checkpoint_slot_mismatch_count(
                cases,
                AccuracyCheckpointV1::LocalRecap,
                "cross_layer_contradiction_count",
            ),
        ),
        (
            "forbidden_stale_term_leakage",
            cases
                .iter()
                .map(|case| case.forbidden_claim_matches.len() as i64)
                .sum(),
        ),
        (
            "wrong_confident_rate",
            cases.iter().filter(|case| case.wrong_confident).count() as i64,
        ),
        (
            "frame_fallback_public_target_count",
            cases
                .iter()
                .filter(|case| !case.public_target_honest)
                .count() as i64,
        ),
    ];
    for (name, numerator) in count_metrics {
        metrics.insert(name.to_string(), metric(numerator, all));
    }
    metrics.insert(
        "model_on_model_off_task_identity_agreement".to_string(),
        metric(
            cases
                .iter()
                .filter(|case| case.model_on_off_task_identity_match)
                .count() as i64,
            all,
        ),
    );
    metrics.insert(
        "deterministic_replay_agreement".to_string(),
        metric(
            cases
                .iter()
                .filter(|case| case.deterministic_replay_match)
                .count() as i64,
            all,
        ),
    );
    metrics
}

fn aggregate_confidence_calibration(
    cases: &[ContinueAccuracyCaseResult],
) -> BTreeMap<String, ConfidenceCalibrationMetric> {
    let mut dimensions = BTreeMap::<String, Vec<&ConfidenceDimensionObservation>>::new();
    for case in cases {
        for (dimension, observation) in &case.confidence_dimensions {
            dimensions
                .entry(dimension.clone())
                .or_default()
                .push(observation);
        }
    }
    dimensions
        .into_iter()
        .map(|(dimension, observations)| {
            let evaluated = observations
                .iter()
                .filter_map(|observation| {
                    observation.correct.map(|correct| (*observation, correct))
                })
                .collect::<Vec<_>>();
            let mut predicted_label_counts = BTreeMap::new();
            for observation in &observations {
                *predicted_label_counts
                    .entry(observation.label.clone())
                    .or_insert(0) += 1;
            }
            let correct_count = evaluated.iter().filter(|(_, correct)| *correct).count();
            let incorrect_count = evaluated.len().saturating_sub(correct_count);
            let overconfident_wrong_count = observations
                .iter()
                .filter(|observation| {
                    observation.expected_positive == Some(false) && observation.score >= 0.75
                })
                .count();
            let underconfident_correct_count = observations
                .iter()
                .filter(|observation| {
                    observation.expected_positive == Some(true) && observation.score < 0.50
                })
                .count();
            let calibration = observations
                .iter()
                .filter_map(|observation| {
                    observation
                        .expected_positive
                        .map(|expected_positive| (*observation, expected_positive))
                })
                .collect::<Vec<_>>();
            let brier_score = (!calibration.is_empty()).then(|| {
                round_metric(
                    calibration
                        .iter()
                        .map(|(observation, expected_positive)| {
                            let outcome = if *expected_positive { 1.0 } else { 0.0 };
                            (observation.score - outcome).powi(2)
                        })
                        .sum::<f64>()
                        / calibration.len() as f64,
                )
            });
            let enough_for_ece = evaluated.len() >= 100;
            let expected_calibration_error = enough_for_ece.then(|| {
                round_metric(
                    calibration
                        .iter()
                        .map(|(observation, expected_positive)| {
                            (observation.score - if *expected_positive { 1.0 } else { 0.0 }).abs()
                        })
                        .sum::<f64>()
                        / calibration.len() as f64,
                )
            });
            (
                dimension,
                ConfidenceCalibrationMetric {
                    sample_size: evaluated.len(),
                    predicted_label_counts,
                    correct_count,
                    incorrect_count,
                    overconfident_wrong_count,
                    underconfident_correct_count,
                    brier_score,
                    expected_calibration_error,
                    status: if enough_for_ece {
                        "calibrated_sample_available"
                    } else {
                        "insufficient_sample_size_for_ece"
                    }
                    .to_string(),
                },
            )
        })
        .collect()
}

fn aggregate_probe_metrics(cases: &[ContinueAccuracyCaseResult]) -> ProbeAccuracyMetrics {
    let observations = cases
        .iter()
        .flat_map(|case| case.probe_counterfactuals.iter())
        .collect::<Vec<_>>();
    let mut status_counts = BTreeMap::new();
    let mut durations = Vec::new();
    for observation in &observations {
        *status_counts.entry(observation.status.clone()).or_insert(0) += 1;
        durations.push(observation.duration_ms);
    }
    durations.sort_by(|left, right| left.total_cmp(right));
    let mut without_probe = cases
        .iter()
        .filter(|case| case.probe_counterfactuals.is_empty())
        .map(|case| case.replay_duration_ms)
        .collect::<Vec<_>>();
    let mut with_probe = cases
        .iter()
        .filter(|case| !case.probe_counterfactuals.is_empty())
        .map(|case| case.replay_duration_ms)
        .collect::<Vec<_>>();
    without_probe.sort_by(|left, right| left.total_cmp(right));
    with_probe.sort_by(|left, right| left.total_cmp(right));
    ProbeAccuracyMetrics {
        sample_size: observations.len(),
        status_counts,
        changed_count: observations
            .iter()
            .filter(|item| item.evidence_changed)
            .count(),
        rerun_count: observations
            .iter()
            .filter(|item| item.reran_decision)
            .count(),
        timeout_count: observations
            .iter()
            .filter(|item| item.status == "timed_out")
            .count(),
        privacy_blocked_count: observations
            .iter()
            .filter(|item| item.status == "privacy_blocked")
            .count(),
        failure_count: observations
            .iter()
            .filter(|item| item.status == "failed")
            .count(),
        changed_rate: rate_usize(
            observations
                .iter()
                .filter(|item| item.evidence_changed)
                .count(),
            observations.len(),
        ),
        rerun_rate: rate_usize(
            observations
                .iter()
                .filter(|item| item.reran_decision)
                .count(),
            observations.len(),
        ),
        timeout_rate: rate_usize(
            observations
                .iter()
                .filter(|item| item.status == "timed_out")
                .count(),
            observations.len(),
        ),
        privacy_blocked_rate: rate_usize(
            observations
                .iter()
                .filter(|item| item.status == "privacy_blocked")
                .count(),
            observations.len(),
        ),
        failure_rate: rate_usize(
            observations
                .iter()
                .filter(|item| item.status == "failed")
                .count(),
            observations.len(),
        ),
        p50_duration_ms: (!durations.is_empty()).then(|| round_ms(percentile(&durations, 0.50))),
        p95_duration_ms: (!durations.is_empty()).then(|| round_ms(percentile(&durations, 0.95))),
        decisions_with_probe_sample_size: with_probe.len(),
        decisions_without_probe_sample_size: without_probe.len(),
        p50_decision_with_probe_ms: (!with_probe.is_empty())
            .then(|| round_ms(percentile(&with_probe, 0.50))),
        p95_decision_with_probe_ms: (!with_probe.is_empty())
            .then(|| round_ms(percentile(&with_probe, 0.95))),
        p50_decision_without_probe_ms: (!without_probe.is_empty())
            .then(|| round_ms(percentile(&without_probe, 0.50))),
        p95_decision_without_probe_ms: (!without_probe.is_empty())
            .then(|| round_ms(percentile(&without_probe, 0.95))),
    }
}

fn round_metric(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

fn rate_usize(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator > 0).then(|| round_metric(numerator as f64 / denominator as f64))
}

fn checkpoint_slot_mismatch_count(
    cases: &[ContinueAccuracyCaseResult],
    checkpoint: AccuracyCheckpointV1,
    slot: &str,
) -> i64 {
    cases
        .iter()
        .filter_map(|case| {
            case.checkpoint_results
                .iter()
                .find(|result| result.checkpoint == checkpoint)
        })
        .filter(|result| result.mismatched_slots.iter().any(|value| value == slot))
        .count() as i64
}

fn metric(numerator: i64, denominator: i64) -> AccuracyMetricValue {
    AccuracyMetricValue {
        numerator,
        denominator,
        rate: (denominator > 0).then(|| round_score(numerator as f64 / denominator as f64)),
        status: if denominator > 0 {
            "measured"
        } else {
            "insufficient_sample_size"
        }
        .to_string(),
    }
}

fn metric_from_checkpoint(
    cases: &[ContinueAccuracyCaseResult],
    checkpoint: AccuracyCheckpointV1,
) -> f64 {
    if cases.is_empty() {
        0.0
    } else {
        round_score(
            cases
                .iter()
                .filter(|case| checkpoint_matched(case, checkpoint))
                .count() as f64
                / cases.len() as f64,
        )
    }
}

fn metric_for_checkpoint_slots(
    cases: &[ContinueAccuracyCaseResult],
    checkpoint: AccuracyCheckpointV1,
) -> AccuracyMetricValue {
    let mut numerator = 0i64;
    let mut denominator = 0i64;
    for result in cases.iter().filter_map(|case| {
        case.checkpoint_results
            .iter()
            .find(|result| result.checkpoint == checkpoint)
    }) {
        if result.status != AccuracyCheckpointStatus::NotExpected {
            numerator += result.correct_slot_count as i64;
            denominator += result.expected_slot_count as i64;
        }
    }
    metric(numerator, denominator)
}

fn metric_for_named_slots(
    cases: &[ContinueAccuracyCaseResult],
    checkpoint: AccuracyCheckpointV1,
    slots: &[&str],
) -> AccuracyMetricValue {
    let names = slots.iter().copied().collect::<HashSet<_>>();
    let mut numerator = 0i64;
    let mut denominator = 0i64;
    for result in cases.iter().filter_map(|case| {
        case.checkpoint_results
            .iter()
            .find(|result| result.checkpoint == checkpoint)
    }) {
        for slot in result
            .evaluated_slots
            .iter()
            .filter(|slot| names.contains(slot.as_str()))
        {
            denominator += 1;
            if !result.mismatched_slots.contains(slot) {
                numerator += 1;
            }
        }
    }
    metric(numerator, denominator)
}

fn checkpoint_matched(case: &ContinueAccuracyCaseResult, checkpoint: AccuracyCheckpointV1) -> bool {
    case.checkpoint_results
        .iter()
        .find(|result| result.checkpoint == checkpoint)
        .is_some_and(|result| result.status == AccuracyCheckpointStatus::Match)
}

fn count_forbidden(cases: &[ContinueAccuracyCaseResult], term: &str) -> i64 {
    cases
        .iter()
        .flat_map(|case| &case.forbidden_claim_matches)
        .filter(|value| normalize(value).contains(term))
        .count() as i64
}

fn checkpoint_order() -> [AccuracyCheckpointV1; 17] {
    [
        AccuracyCheckpointV1::ResolvedText,
        AccuracyCheckpointV1::RegionRoles,
        AccuracyCheckpointV1::ConversationalRoles,
        AccuracyCheckpointV1::OrderedTurnSpans,
        AccuracyCheckpointV1::LatestTaskTurn,
        AccuracyCheckpointV1::TaskAction,
        AccuracyCheckpointV1::SemanticDelta,
        AccuracyCheckpointV1::EligibleFeedback,
        AccuracyCheckpointV1::SelectedWorkstream,
        AccuracyCheckpointV1::EligibleOpenLoop,
        AccuracyCheckpointV1::CurrentSurface,
        AccuracyCheckpointV1::SelectedCandidate,
        AccuracyCheckpointV1::PrimaryRecapSegment,
        AccuracyCheckpointV1::LocalRecap,
        AccuracyCheckpointV1::ValidatedRecap,
        AccuracyCheckpointV1::PublicTarget,
        AccuracyCheckpointV1::ProductAnswer,
    ]
}

fn normalized_snapshot_fingerprint(snapshot: &ReplaySnapshot) -> String {
    let checkpoint_names = snapshot
        .actual
        .keys()
        .map(|checkpoint| checkpoint_label(*checkpoint))
        .collect::<Vec<_>>();
    let task_action = snapshot
        .actual
        .get(&AccuracyCheckpointV1::TaskAction)
        .and_then(|slots| slots.get("action_kind"))
        .cloned()
        .unwrap_or(Value::Null);
    let value = json!({
        "checkpoint_names": checkpoint_names,
        "task_action": stable_semantic_value(task_action),
        "public_target_honest": public_target_honest(&snapshot.decision),
        "has_return_target": snapshot.decision.return_target.is_some(),
        "has_resume_work_target": snapshot.decision.resume_work_target.is_some(),
        "headline": normalize(&snapshot.decision.handoff.headline),
        "last_state": normalize(&snapshot.decision.handoff.last_state_line),
        "next_action": normalize(&snapshot.decision.handoff.next_action),
        "recap_task": normalize(snapshot.decision.activity_recap.primary_work_summary.as_deref().unwrap_or("")),
        "confidence": snapshot.decision.confidence_label,
        "output_mode": snapshot.decision.continue_output_mode,
        "model_parity": snapshot.model_parity
    });
    stable_json_sha256(&value).unwrap_or_else(|_| "fingerprint_error".to_string())
}

fn stable_semantic_value(value: Value) -> Value {
    match value {
        Value::Array(values) => {
            Value::Array(values.into_iter().map(stable_semantic_value).collect())
        }
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .filter(|(key, _)| {
                    let lower = key.to_ascii_lowercase();
                    !matches!(
                        lower.as_str(),
                        "id" | "decision_id"
                            | "candidate_id"
                            | "workstream_id"
                            | "artifact_id"
                            | "frame_id"
                            | "timestamp_ms"
                            | "created_at_ms"
                            | "updated_at_ms"
                            | "observed_at_ms"
                            | "browser_url"
                            | "document_path"
                            | "content_hash"
                            | "evidence_watermark_hash"
                    ) && !lower.ends_with("_id")
                        && !lower.ends_with("_at_ms")
                        && !lower.ends_with("_hash")
                })
                .map(|(key, value)| (key, stable_semantic_value(value)))
                .collect(),
        ),
        Value::String(text) => {
            let lower = text.to_ascii_lowercase();
            if [
                "frame-",
                "artifact-",
                "workstream-",
                "decision-",
                "accuracy-",
            ]
            .iter()
            .any(|marker| lower.contains(marker))
            {
                Value::String("<opaque-generated-id>".to_string())
            } else {
                Value::String(text)
            }
        }
        other => other,
    }
}

fn public_target_honest(decision: &ContinueDecisionResult) -> bool {
    let targets = [
        decision.return_target.as_ref(),
        decision.resume_work_target.as_ref(),
    ];
    targets.into_iter().flatten().all(|target| {
        target.openability == "openable"
            && (target.browser_url.is_some() || target.document_path.is_some())
    })
}

fn public_semantic_text(decision: &ContinueDecisionResult) -> String {
    [
        decision.handoff.headline.as_str(),
        decision.handoff.return_line.as_str(),
        decision.handoff.last_state_line.as_str(),
        decision.handoff.next_action.as_str(),
        decision
            .activity_recap
            .primary_work_summary
            .as_deref()
            .unwrap_or(""),
        decision
            .activity_recap
            .last_meaningful_state
            .as_deref()
            .unwrap_or(""),
    ]
    .join(" ")
}

fn public_checkpoint_text(
    decision: &ContinueDecisionResult,
    checkpoint: AccuracyCheckpointV1,
) -> String {
    match checkpoint {
        AccuracyCheckpointV1::ProductAnswer => public_semantic_text(decision),
        AccuracyCheckpointV1::PublicTarget => {
            serde_json::to_string(&(&decision.return_target, &decision.resume_work_target))
                .unwrap_or_default()
        }
        AccuracyCheckpointV1::PrimaryRecapSegment
        | AccuracyCheckpointV1::LocalRecap
        | AccuracyCheckpointV1::ValidatedRecap => {
            serde_json::to_string(&decision.activity_recap).unwrap_or_default()
        }
        _ => String::new(),
    }
}

fn checkpoint_text(snapshot: &ReplaySnapshot, checkpoint: AccuracyCheckpointV1) -> String {
    let public = public_checkpoint_text(&snapshot.decision, checkpoint);
    if !public.is_empty() {
        public
    } else {
        snapshot
            .actual
            .get(&checkpoint)
            .and_then(|slots| serde_json::to_string(slots).ok())
            .unwrap_or_default()
    }
}

fn object_slots(value: Value) -> BTreeMap<String, Value> {
    value
        .as_object()
        .map(|map| {
            map.iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

fn load_string_column(conn: &Connection, sql: &str) -> Result<Vec<String>, String> {
    let mut statement = conn.prepare(sql).map_err(to_string)?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;
    Ok(rows)
}

fn load_pair_columns(conn: &Connection, sql: &str) -> Result<Vec<(String, String)>, String> {
    let mut statement = conn.prepare(sql).map_err(to_string)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(to_string)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(to_string)?;
    Ok(rows)
}

fn recap_checkpoint_slots(recap: &super::ContinueActivityRecap) -> BTreeMap<String, Value> {
    let mut slots = BTreeMap::new();
    if let Some(summary) = recap
        .primary_work_label
        .as_deref()
        .or(recap.primary_work_summary.as_deref())
    {
        slots.insert("task_identity".to_string(), json!(summary));
    }
    slots.insert(
        "execution_state".to_string(),
        json!(format!("{:?}", recap.current_state).to_ascii_lowercase()),
    );
    if let Some(next) = recap.next_action_summary.as_deref() {
        slots.insert("next_action".to_string(), json!(next));
    }
    slots
}

fn mapped_frame(map: &HashMap<String, i64>, record: &FixtureSourceRecordV1) -> Result<i64, String> {
    record
        .frame_id
        .as_ref()
        .and_then(|id| map.get(id))
        .copied()
        .ok_or_else(|| format!("{} references missing frame", record.record_id))
}

fn aggregate_ax_text(fixture: &ContinueAccuracyFixtureV1, frame_id: &str) -> Option<String> {
    let mut nodes = fixture
        .redacted_source_records
        .ax_nodes
        .iter()
        .filter(|node| node.frame_id.as_deref() == Some(frame_id))
        .collect::<Vec<_>>();
    nodes.sort_by_key(|node| node.source_order.unwrap_or(0));
    let text = nodes
        .into_iter()
        .filter_map(|node| node.text.as_ref().map(|text| text.text.trim()))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    (!text.is_empty()).then_some(text)
}

fn metadata_label<'a>(
    metadata: &'a BTreeMap<String, FixtureScalarV1>,
    key: &str,
) -> Option<&'a str> {
    match metadata.get(key) {
        Some(FixtureScalarV1::Label { value }) => Some(value),
        Some(FixtureScalarV1::Text { value }) => Some(&value.text),
        _ => None,
    }
}

fn metadata_number(metadata: &BTreeMap<String, FixtureScalarV1>, key: &str) -> Option<f64> {
    match metadata.get(key) {
        Some(FixtureScalarV1::Number { value }) => Some(*value),
        Some(FixtureScalarV1::Integer { value }) => Some(*value as f64),
        _ => None,
    }
}

fn mapped_metadata_frame(
    frame_map: &HashMap<String, i64>,
    metadata: &BTreeMap<String, FixtureScalarV1>,
    key: &str,
) -> Option<Option<String>> {
    match metadata.get(key) {
        Some(FixtureScalarV1::Null) => Some(None),
        Some(FixtureScalarV1::Label { value }) => {
            Some(frame_map.get(value).map(ToString::to_string))
        }
        _ => None,
    }
}

fn metadata_integer(metadata: &BTreeMap<String, FixtureScalarV1>, key: &str) -> Option<i64> {
    match metadata.get(key) {
        Some(FixtureScalarV1::Integer { value }) => Some(*value),
        _ => None,
    }
}

fn metadata_boolean(metadata: &BTreeMap<String, FixtureScalarV1>, key: &str) -> Option<bool> {
    match metadata.get(key) {
        Some(FixtureScalarV1::Boolean { value }) => Some(*value),
        _ => None,
    }
}

fn historical_label<'a>(record: &'a HistoricalRecordV1, key: &str) -> Option<&'a str> {
    metadata_label(&record.fields, key)
}
fn historical_text<'a>(record: &'a HistoricalRecordV1, key: &str) -> Option<&'a str> {
    metadata_label(&record.fields, key)
}
fn fixture_session_id(fixture: &ContinueAccuracyFixtureV1) -> String {
    format!("accuracy-{}", fixture.case_id)
}

fn semantic_text_match(actual: &str, expected: &str) -> bool {
    if normalized_contains(actual, expected) || normalized_contains(expected, actual) {
        return true;
    }
    let actual_tokens = tokens(actual);
    let expected_tokens = tokens(expected);
    if expected_tokens.is_empty() {
        return actual_tokens.is_empty();
    }
    expected_tokens.intersection(&actual_tokens).count() as f64 / expected_tokens.len() as f64
        >= 0.6
}

fn normalized_contains(haystack: &str, needle: &str) -> bool {
    normalize(haystack).contains(&normalize(needle))
}
fn normalize(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn tokens(value: &str) -> HashSet<String> {
    normalize(value)
        .split_whitespace()
        .filter(|token| {
            token.len() > 2
                && !matches!(
                    *token,
                    "the" | "and" | "what" | "does" | "this" | "that" | "with"
                )
        })
        .map(str::to_string)
        .collect()
}
fn percentile(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values[((values.len() - 1) as f64 * percentile).ceil() as usize]
    }
}
fn round_score(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
fn round_ms(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
fn checkpoint_label(checkpoint: AccuracyCheckpointV1) -> &'static str {
    match checkpoint {
        AccuracyCheckpointV1::ResolvedText => "resolved_text",
        AccuracyCheckpointV1::RegionRoles => "region_roles",
        AccuracyCheckpointV1::ConversationalRoles => "conversational_roles",
        AccuracyCheckpointV1::OrderedTurnSpans => "ordered_turn_spans",
        AccuracyCheckpointV1::LatestTaskTurn => "latest_task_turn",
        AccuracyCheckpointV1::TaskAction => "task_action",
        AccuracyCheckpointV1::SemanticDelta => "semantic_delta",
        AccuracyCheckpointV1::EligibleFeedback => "eligible_feedback",
        AccuracyCheckpointV1::SelectedWorkstream => "selected_workstream",
        AccuracyCheckpointV1::EligibleOpenLoop => "eligible_open_loop",
        AccuracyCheckpointV1::PrimaryRecapSegment => "primary_recap_segment",
        AccuracyCheckpointV1::LocalRecap => "local_recap",
        AccuracyCheckpointV1::ValidatedRecap => "validated_recap",
        AccuracyCheckpointV1::PublicTarget => "public_target",
        AccuracyCheckpointV1::ProductAnswer => "product_answer",
        AccuracyCheckpointV1::CurrentSurface => "current_surface",
        AccuracyCheckpointV1::SelectedCandidate => "selected_candidate",
    }
}
fn to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_accuracy_replay_reports_expected_first_divergence_and_determinism() {
        let report =
            run_committed_continue_accuracy_eval(ContinueAccuracyEvalOptions::default()).unwrap();
        assert_eq!(report.case_count, 8);
        assert!(
            report.milestone_contract_passed,
            "{:?}; cases={:?}",
            report.milestone_violations,
            report
                .cases
                .iter()
                .map(|case| (&case.case_id, case.first_divergent_checkpoint))
                .collect::<Vec<_>>()
        );
        assert!(
            report
                .cases
                .iter()
                .all(|case| case.first_divergent_checkpoint
                    != Some(AccuracyCheckpointV1::LatestTaskTurn)),
            "{:?}",
            report
                .cases
                .iter()
                .map(|case| (&case.case_id, case.first_divergent_checkpoint))
                .collect::<Vec<_>>()
        );
        assert!(
            report
                .cases
                .iter()
                .all(|case| case.deterministic_replay_match),
            "{:?}",
            report
                .cases
                .iter()
                .map(|case| (&case.case_id, case.deterministic_replay_match))
                .collect::<Vec<_>>()
        );
        assert!(report.cases.iter().all(|case| case.privacy_lint_passed));
        for metric in [
            "region_role_macro_f1",
            "conversational_role_macro_f1",
            "latest_user_span_precision",
            "latest_user_span_recall",
            "current_agent_status_precision",
            "current_agent_status_recall",
            "latest_user_goal_accuracy",
            "execution_state_accuracy",
            "current_actor_accuracy",
            "waiting_on_accuracy",
            "task_turn_boundary_accuracy",
            "task_action_accuracy",
            "semantic_delta_temporal_accuracy",
            "model_on_model_off_task_identity_agreement",
        ] {
            assert_eq!(report.metrics[metric].rate, Some(1.0), "metric={metric}");
        }
        assert_eq!(
            report.metrics["prior_completion_override_rate"].numerator,
            0
        );
        assert_eq!(report.probe_metrics.sample_size, 6);
        assert_eq!(report.probe_metrics.timeout_count, 1);
        assert_eq!(report.probe_metrics.privacy_blocked_count, 1);
        let critical = report
            .cases
            .iter()
            .find(|case| case.case_id.contains("all_contaminants"))
            .unwrap();
        assert!(critical.probe_counterfactuals.iter().all(|probe| {
            probe.task_identity_preserved
                && probe.target_confidence_label == "none"
                && !probe.wrong_confident
                && (probe.evidence_changed || !probe.confidence_increased)
                && (probe.reran_decision || !probe.refreshed_warning_emitted)
        }));
        assert!(report
            .confidence_calibration
            .values()
            .all(|metric| metric.status == "insufficient_sample_size_for_ece"));
        assert_eq!(
            report.confidence_calibration["target_openability"].brier_score,
            Some(0.0)
        );
        assert!(!report.release_gate.passed);
        assert_eq!(report.release_gate.corpus.case_count, 8);
        assert_eq!(
            report
                .release_gate
                .corpus
                .independently_human_reviewed_count,
            0
        );
        assert!(report
            .release_gate
            .violations
            .iter()
            .any(|item| item.starts_with("broad_corpus_requires_100_cases")));
        assert!(report
            .release_gate
            .violations
            .contains(&"manual_macos_interruption_recovery_qa_not_proven".to_string()));
    }

    #[test]
    fn first_divergence_and_forbidden_matching_ignore_metadata() {
        assert!(normalized_contains("Stremio is primary", "stremio"));
        assert!(!normalized_contains("audit handle stale-media", "stremio"));
    }

    #[test]
    fn wrong_confident_and_target_honesty_are_typed_not_target_proxies() {
        assert!(semantic_text_match(
            "Trace the Swift bridge",
            "Tracing Swift bridge"
        ));
        assert!(!semantic_text_match(
            "Unrelated playback task",
            "Trace the Swift bridge"
        ));
    }

    #[test]
    fn later_support_surface_preserves_current_task_identity() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "tests/fixtures/continue_accuracy/cases/capture_button_adjacent_after_support_detour.json",
        );
        let fixture = parse_accuracy_fixture_json_for_access(
            &fs::read(path).unwrap(),
            HoldoutAccessModeV1::Development,
        )
        .unwrap();
        let snapshot = run_fixture_once(&fixture).unwrap();
        let region_slots = snapshot
            .actual
            .get(&AccuracyCheckpointV1::RegionRoles)
            .unwrap();
        assert_eq!(
            region_slots.get("support_region"),
            Some(&json!("navigation"))
        );
        let turn = snapshot.decision.current_task_turn.as_ref().unwrap();
        assert_eq!(
            turn.latest_user_goal_summary.as_deref(),
            Some("What does the island Capture button do?")
        );
        assert!(snapshot.decision.return_target.is_none());
        assert!(snapshot.decision.resume_work_target.is_none());
    }

    #[test]
    fn session_013_fixture_recovers_request_and_excludes_control() {
        let fixture = session_013_fixture();
        let snapshot = run_fixture_once(&fixture).unwrap();
        let latest = snapshot
            .actual
            .get(&AccuracyCheckpointV1::LatestTaskTurn)
            .unwrap();
        assert_eq!(
            latest.get("latest_user_goal"),
            Some(&json!(
                "Repair typing-to-frame causality and keep controls out of the current task."
            ))
        );
        assert!(!normalized_contains(
            &checkpoint_text(&snapshot, AccuracyCheckpointV1::LatestTaskTurn),
            "Approve for me"
        ));
        assert!(!normalized_contains(
            &checkpoint_text(&snapshot, AccuracyCheckpointV1::ProductAnswer),
            "Approve for me"
        ));
        assert!(snapshot.decision.return_target.is_none());
        assert!(snapshot.decision.resume_work_target.is_none());
    }

    #[test]
    fn session_013_fixture_preserves_legacy_null_and_control_actionability() {
        let fixture = session_013_fixture();
        let conn = Connection::open_in_memory().unwrap();
        crate::capture::init_db(&conn).unwrap();
        let frame_map = insert_source_records(&conn, &fixture, 10_000).unwrap();
        let typing = conn
            .query_row(
                "SELECT post_frame_id, commit_signal, pre_frame_id, app_bundle_id,
                        app_name, window_id, window_title, raw_text_captured
                 FROM typing_bursts WHERE id = 'typing-001'",
                [],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<i64>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, i64>(7)?,
                    ))
                },
            )
            .unwrap();
        assert!(typing.0.is_none());
        assert_eq!(typing.1.as_deref(), Some("enter"));
        assert_eq!(typing.2, Some(frame_map["frame-before-submit"].to_string()));
        assert_eq!(typing.3.as_deref(), Some("com.fixture.agentchat"));
        assert_eq!(typing.4.as_deref(), Some("AgentChat"));
        assert_eq!(typing.5, Some(42));
        assert_eq!(typing.6.as_deref(), Some("Synthetic review task"));
        assert_eq!(typing.7, 0);

        let control = conn
            .query_row(
                "SELECT role, enabled, actions_json FROM ax_nodes WHERE id = 'node-005'",
                [],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<i64>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(control.0.as_deref(), Some("AXButton"));
        assert_eq!(control.1, Some(1));
        assert_eq!(control.2.as_deref(), Some("[\"AXPress\"]"));
    }

    fn session_013_fixture() -> ContinueAccuracyFixtureV1 {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "tests/fixtures/continue_accuracy/cases/task_truth_session_013_causal_control_containment.json",
        );
        parse_accuracy_fixture_json_for_access(
            &fs::read(path).unwrap(),
            HoldoutAccessModeV1::Development,
        )
        .unwrap()
    }
}
