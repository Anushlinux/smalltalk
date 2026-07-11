use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const FINAL_SCHEMA: &str = "smalltalk.task_truth_v2.final_release_report.v1";
const EVALUATOR_SCHEMA: &str = "smalltalk.task_truth_v2.report.v1";
const MANUAL_SCHEMA: &str = "smalltalk.task_truth_v2.manual_macos_qa.v1";
const PERFORMANCE_SCHEMA: &str = "smalltalk.task_truth_v2.performance_cost_privacy.v1";
const BUDGET_SCHEMA: &str = "smalltalk.task_truth_v2.release_budgets.v1";
const BUDGET_POLICY_VERSION: &str = "tt2.05-budgets-v1";
const POLICY_VERSION: &str = "tt2.02-v1";
const REQUIRED_LIVE: usize = 200;
const REQUIRED_HOLDOUT: usize = 50;
const REQUIRED_SURFACE_CASES: u64 = 15;

const REQUIRED_METRICS: [&str; 13] = [
    "wrong_primary_task_rate",
    "control_navigation_as_task_rate",
    "useful_non_generic_task_summary",
    "task_object_accuracy",
    "execution_state_accuracy",
    "supported_next_action_precision",
    "supported_next_action_coverage",
    "return_target_precision",
    "unsupported_specific_claim_rate",
    "stronger_manual_result_downgraded",
    "unseen_application_useful_summary",
    "human_immediately_useful",
    "model_on_off_unexplained_task_disagreement",
];

const REQUIRED_SURFACES: [&str; 10] = [
    "agent_chat",
    "editor_ide",
    "terminal",
    "browser_research_search",
    "documents",
    "spreadsheets",
    "email_messaging",
    "pdf_file_manager",
    "custom_rendered_canvas",
    "mixed_window_thin_unknown",
];

const REQUIRED_MANUAL_SCENARIOS: [&str; 14] = [
    "older_completed_then_new_question",
    "weak_ax_chat_submit",
    "code_command_error_switch",
    "document_or_spreadsheet_switch",
    "research_with_older_openable_distraction",
    "waiting_output_interruption",
    "custom_rendered_or_thin_accessibility",
    "privacy_blocking",
    "two_close_tasks_choice_ui",
    "task_understood_target_unavailable",
    "selected_task_only_direct_open",
    "background_cannot_downgrade_manual",
    "main_card_island_parity",
    "not_right_feedback_scoped",
];

fn main() {
    if let Err(error) = run() {
        eprintln!("Task Truth v2 release gate failed: {error}");
        std::process::exit(1);
    }
}

fn read_json(path: &Path) -> Result<Value, String> {
    serde_json::from_slice(&fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?)
        .map_err(|error| format!("{}: {error}", path.display()))
}

fn string_at<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

fn u64_at(value: &Value, pointer: &str) -> Option<u64> {
    value.pointer(pointer).and_then(Value::as_u64)
}

fn f64_at(value: &Value, pointer: &str) -> Option<f64> {
    value.pointer(pointer).and_then(Value::as_f64)
}

fn bool_at(value: &Value, pointer: &str) -> Option<bool> {
    value.pointer(pointer).and_then(Value::as_bool)
}

fn validate_evaluator(value: &Value) -> Vec<String> {
    let mut violations = Vec::new();
    if string_at(value, "/schema") != Some(EVALUATOR_SCHEMA) {
        violations.push("evaluator_schema_invalid".into());
    }
    if string_at(value, "/policy_version") != Some(POLICY_VERSION) {
        violations.push("evaluator_policy_version_invalid".into());
    }
    if bool_at(value, "/release_gate_passed") != Some(true) {
        violations.push("evaluator_release_gate_closed".into());
    }
    if value
        .get("release_gate_violations")
        .and_then(Value::as_array)
        .is_none_or(|items| !items.is_empty())
    {
        violations.push("evaluator_has_violations".into());
    }
    let cases = value
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let reviewed = cases
        .iter()
        .filter(|case| case.get("release_eligible").and_then(Value::as_bool) == Some(true))
        .count();
    if reviewed < REQUIRED_LIVE {
        violations.push(format!(
            "independently_reviewed_live_requires_{REQUIRED_LIVE}_found_{reviewed}"
        ));
    }
    let holdout = cases
        .iter()
        .filter(|case| {
            case.get("release_eligible").and_then(Value::as_bool) == Some(true)
                && case.get("partition").and_then(Value::as_str) == Some("locked_holdout")
        })
        .count();
    if holdout < REQUIRED_HOLDOUT {
        violations.push(format!(
            "locked_holdout_requires_{REQUIRED_HOLDOUT}_found_{holdout}"
        ));
    }
    for metric in REQUIRED_METRICS {
        let pointer = format!("/tt2_05_metric_results/{metric}");
        let assessment = value.pointer(&pointer);
        if assessment
            .and_then(|item| item.get("passed"))
            .and_then(Value::as_bool)
            != Some(true)
            || assessment
                .and_then(|item| item.get("denominator"))
                .and_then(Value::as_u64)
                .is_none_or(|denominator| denominator == 0)
        {
            violations.push(format!("tt2_05_metric_{metric}_not_proven"));
        }
        let interval_pointer = format!("/tt2_05_confidence_intervals/{metric}");
        let interval = value.pointer(&interval_pointer);
        if interval
            .and_then(|item| item.get("method"))
            .and_then(Value::as_str)
            != Some("wilson_score")
            || interval
                .and_then(|item| item.get("lower"))
                .and_then(Value::as_f64)
                .is_none()
            || interval
                .and_then(|item| item.get("upper"))
                .and_then(Value::as_f64)
                .is_none()
        {
            violations.push(format!(
                "tt2_05_metric_{metric}_confidence_interval_missing"
            ));
        }
    }
    for surface in REQUIRED_SURFACES {
        let pointer = format!("/tt2_05_surface_wrong_task_results/{surface}");
        let assessment = value.pointer(&pointer);
        let denominator = assessment
            .and_then(|item| item.get("denominator"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if assessment
            .and_then(|item| item.get("passed"))
            .and_then(Value::as_bool)
            != Some(true)
            || denominator < REQUIRED_SURFACE_CASES
        {
            violations.push(format!("surface_{surface}_wrong_task_gate_not_proven"));
        }
        let interval_pointer =
            format!("/tt2_05_confidence_intervals/wrong_primary_task_rate.surface.{surface}");
        let interval = value.pointer(&interval_pointer);
        if interval
            .and_then(|item| item.get("method"))
            .and_then(Value::as_str)
            != Some("wilson_score")
            || interval
                .and_then(|item| item.get("lower"))
                .and_then(Value::as_f64)
                .is_none()
            || interval
                .and_then(|item| item.get("upper"))
                .and_then(Value::as_f64)
                .is_none()
        {
            violations.push(format!(
                "surface_{surface}_wrong_task_confidence_interval_missing"
            ));
        }
    }
    for (slice, minimum) in [
        ("interruption_resumption", 30_u64),
        ("ambiguous_or_privacy_blocked", 20),
        ("waiting_on_agent_or_application", 20),
        ("completed_vs_new_task", 20),
    ] {
        let found =
            u64_at(value, &format!("/reviewed_live_slice_denominators/{slice}")).unwrap_or(0);
        if found < minimum {
            violations.push(format!("slice_{slice}_requires_{minimum}_found_{found}"));
        }
    }
    violations
}

fn validate_manual_qa(value: Option<&Value>) -> Vec<String> {
    let Some(value) = value else {
        return vec!["manual_macos_qa_manifest_missing".into()];
    };
    let mut violations = Vec::new();
    if string_at(value, "/schema") != Some(MANUAL_SCHEMA) {
        violations.push("manual_macos_qa_schema_invalid".into());
    }
    let scenarios = value
        .get("scenarios")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut seen = BTreeSet::new();
    for scenario in &scenarios {
        let id = scenario
            .get("scenario_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !seen.insert(id.to_string()) {
            violations.push(format!("manual_scenario_{id}_duplicated"));
        }
        let complete = scenario.get("status").and_then(Value::as_str) == Some("passed")
            && scenario
                .get("reviewer")
                .and_then(Value::as_str)
                .is_some_and(|text| !text.trim().is_empty())
            && scenario
                .get("app_build_commit")
                .and_then(Value::as_str)
                .is_some_and(|text| !text.trim().is_empty())
            && scenario
                .get("expected_result")
                .and_then(Value::as_str)
                .is_some_and(|text| !text.trim().is_empty())
            && scenario
                .get("actual_result")
                .and_then(Value::as_str)
                .is_some_and(|text| !text.trim().is_empty())
            && scenario
                .get("evidence_ids")
                .and_then(Value::as_array)
                .is_some_and(|ids| {
                    !ids.is_empty()
                        && ids.iter().all(|id| {
                            id.as_str().is_some_and(|text| {
                                !text.trim().is_empty()
                                    && !text.contains("/Users/")
                                    && !text.contains("private/")
                            })
                        })
                });
        if !complete {
            violations.push(format!("manual_scenario_{id}_incomplete"));
        }
    }
    for required in REQUIRED_MANUAL_SCENARIOS {
        if !seen.contains(required) {
            violations.push(format!("manual_scenario_{required}_missing"));
        }
    }
    if scenarios.len() != REQUIRED_MANUAL_SCENARIOS.len() {
        violations.push(format!(
            "manual_scenario_count_expected_{}_found_{}",
            REQUIRED_MANUAL_SCENARIOS.len(),
            scenarios.len()
        ));
    }
    violations
}

fn validate_budgets(value: Option<&Value>, baseline_fingerprint: &str) -> Vec<String> {
    let Some(value) = value else {
        return vec!["release_budget_policy_missing".into()];
    };
    let mut violations = Vec::new();
    if string_at(value, "/schema") != Some(BUDGET_SCHEMA)
        || string_at(value, "/policy_version") != Some(BUDGET_POLICY_VERSION)
    {
        violations.push("release_budget_policy_schema_or_version_invalid".into());
    }
    if bool_at(value, "/frozen_before_holdout_access") != Some(true)
        || bool_at(value, "/holdout_accessed_at_freeze") != Some(false)
        || string_at(value, "/baseline_report_id") != Some(baseline_fingerprint)
    {
        violations.push("release_budget_policy_not_frozen_from_baseline".into());
    }
    for field in [
        "manual_continue_p50_ms_max",
        "manual_continue_p95_ms_max",
        "local_packet_checkpoint_p50_ms_max",
        "local_packet_checkpoint_p95_ms_max",
        "model_request_max_images",
        "model_request_max_bytes",
        "model_request_max_tokens",
        "semantic_checkpoint_writes_per_minute_max",
        "semantic_checkpoint_retention_count_max",
        "provider_failure_rate_max",
        "safe_fallback_rate_min",
    ] {
        if f64_at(value, &format!("/budgets/{field}")).is_none_or(|number| number < 0.0) {
            violations.push(format!("release_budget_{field}_missing"));
        }
    }
    if f64_at(value, "/budgets/manual_continue_p50_ms_max")
        > f64_at(value, "/budgets/manual_continue_p95_ms_max")
        || f64_at(value, "/budgets/local_packet_checkpoint_p50_ms_max")
            > f64_at(value, "/budgets/local_packet_checkpoint_p95_ms_max")
        || f64_at(value, "/budgets/provider_failure_rate_max").is_some_and(|rate| rate > 1.0)
        || f64_at(value, "/budgets/safe_fallback_rate_min").is_some_and(|rate| rate > 1.0)
    {
        violations.push("release_budget_values_invalid".into());
    }
    if f64_at(value, "/budgets/model_request_max_images").is_some_and(|limit| limit > 4.0)
        || f64_at(value, "/budgets/model_request_max_bytes")
            .is_some_and(|limit| limit > 12_582_912.0)
        || f64_at(value, "/budgets/semantic_checkpoint_retention_count_max")
            .is_some_and(|limit| limit > 500.0)
    {
        violations.push("release_budget_exceeds_architectural_privacy_or_retention_cap".into());
    }
    violations
}

fn measurement_within(
    value: &Value,
    budgets: &Value,
    measurement: &str,
    budget: &str,
    higher_is_better: bool,
) -> bool {
    let actual = f64_at(value, &format!("/measurements/{measurement}"));
    let limit = f64_at(budgets, &format!("/budgets/{budget}"));
    actual.zip(limit).is_some_and(|(actual, limit)| {
        if higher_is_better {
            actual >= limit
        } else {
            actual <= limit
        }
    })
}

fn validate_performance(value: Option<&Value>, budgets: Option<&Value>) -> Vec<String> {
    let Some(value) = value else {
        return vec!["performance_cost_privacy_manifest_missing".into()];
    };
    let mut violations = Vec::new();
    if string_at(value, "/schema") != Some(PERFORMANCE_SCHEMA) {
        violations.push("performance_cost_privacy_schema_invalid".into());
    }
    if string_at(value, "/budget_policy_version") != Some(BUDGET_POLICY_VERSION) {
        violations.push("performance_budget_policy_version_mismatch".into());
    }
    if bool_at(value, "/policy_frozen_before_holdout_access") != Some(true)
        || bool_at(value, "/holdout_accessed_at_freeze") != Some(false)
    {
        violations.push("performance_budget_not_frozen_before_holdout".into());
    }
    if u64_at(value, "/sample_count").unwrap_or(0) < 30 {
        violations.push("performance_sample_count_requires_30".into());
    }
    for measurement in [
        "manual_continue_p50_ms",
        "manual_continue_p95_ms",
        "local_packet_checkpoint_p50_ms",
        "local_packet_checkpoint_p95_ms",
        "model_request_max_images",
        "model_request_max_bytes",
        "model_request_max_tokens",
        "semantic_checkpoint_writes_per_minute",
        "semantic_checkpoint_retention_count",
        "provider_failure_rate",
        "safe_fallback_rate",
    ] {
        if f64_at(value, &format!("/measurements/{measurement}")).is_none_or(|number| number < 0.0)
        {
            violations.push(format!("performance_measurement_{measurement}_invalid"));
        }
    }
    if f64_at(value, "/measurements/manual_continue_p50_ms")
        > f64_at(value, "/measurements/manual_continue_p95_ms")
        || f64_at(value, "/measurements/local_packet_checkpoint_p50_ms")
            > f64_at(value, "/measurements/local_packet_checkpoint_p95_ms")
        || f64_at(value, "/measurements/provider_failure_rate").is_some_and(|rate| rate > 1.0)
        || f64_at(value, "/measurements/safe_fallback_rate").is_some_and(|rate| rate > 1.0)
    {
        violations.push("performance_measurement_values_invalid".into());
    }
    for (measurement, budget, higher_is_better) in [
        (
            "manual_continue_p50_ms",
            "manual_continue_p50_ms_max",
            false,
        ),
        (
            "manual_continue_p95_ms",
            "manual_continue_p95_ms_max",
            false,
        ),
        (
            "local_packet_checkpoint_p50_ms",
            "local_packet_checkpoint_p50_ms_max",
            false,
        ),
        (
            "local_packet_checkpoint_p95_ms",
            "local_packet_checkpoint_p95_ms_max",
            false,
        ),
        (
            "model_request_max_images",
            "model_request_max_images",
            false,
        ),
        ("model_request_max_bytes", "model_request_max_bytes", false),
        (
            "model_request_max_tokens",
            "model_request_max_tokens",
            false,
        ),
        (
            "semantic_checkpoint_writes_per_minute",
            "semantic_checkpoint_writes_per_minute_max",
            false,
        ),
        (
            "semantic_checkpoint_retention_count",
            "semantic_checkpoint_retention_count_max",
            false,
        ),
        ("provider_failure_rate", "provider_failure_rate_max", false),
        ("safe_fallback_rate", "safe_fallback_rate_min", true),
    ] {
        if budgets.is_none_or(|budgets| {
            !measurement_within(value, budgets, measurement, budget, higher_is_better)
        }) {
            violations.push(format!(
                "performance_budget_{measurement}_failed_or_missing"
            ));
        }
    }
    for (pointer, violation) in [
        (
            "/background_multimodal_requests",
            "background_multimodal_requests_nonzero",
        ),
        ("/privacy_violations", "privacy_violations_nonzero"),
        ("/unsafe_opens", "unsafe_opens_nonzero"),
        (
            "/committed_fixture_secret_findings",
            "committed_fixture_secret_findings_nonzero",
        ),
    ] {
        if u64_at(value, pointer) != Some(0) {
            violations.push(violation.into());
        }
    }
    if bool_at(value, "/privacy_blocked_frames_excluded_before_transport") != Some(true) {
        violations.push("privacy_blocked_transport_exclusion_not_proven".into());
    }
    violations
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mut input = PathBuf::from(
        "tests/fixtures/continue_accuracy/task_truth_v2/release-evaluator-report.v1.json",
    );
    let mut baseline =
        PathBuf::from("tests/fixtures/continue_accuracy/task_truth_v2/baseline-report.v1.json");
    let mut output = PathBuf::from(
        "tests/fixtures/continue_accuracy/task_truth_v2/final-release-report.v1.json",
    );
    let mut manual_qa =
        PathBuf::from("tests/fixtures/continue_accuracy/task_truth_v2/manual-macos-qa.v1.json");
    let mut performance = PathBuf::from(
        "tests/fixtures/continue_accuracy/task_truth_v2/performance-cost-privacy.v1.json",
    );
    let mut budgets =
        PathBuf::from("tests/fixtures/continue_accuracy/task_truth_v2/release-budgets.v1.json");
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--input" => input = PathBuf::from(args.next().ok_or("--input requires a path")?),
            "--baseline" => {
                baseline = PathBuf::from(args.next().ok_or("--baseline requires a path")?)
            }
            "--output" => output = PathBuf::from(args.next().ok_or("--output requires a path")?),
            "--manual-qa" => {
                manual_qa = PathBuf::from(args.next().ok_or("--manual-qa requires a path")?)
            }
            "--performance" => {
                performance = PathBuf::from(args.next().ok_or("--performance requires a path")?)
            }
            "--budgets" => budgets = PathBuf::from(args.next().ok_or("--budgets requires a path")?),
            other => return Err(format!("unknown argument {other}")),
        }
    }

    let evaluator = read_json(&input)?;
    let baseline_bytes =
        fs::read(&baseline).map_err(|error| format!("{}: {error}", baseline.display()))?;
    let baseline_fingerprint = smalltalk_lib::task_truth_v2_baseline_sha256(&baseline_bytes);
    let manual_value = manual_qa
        .exists()
        .then(|| read_json(&manual_qa))
        .transpose()?;
    let performance_value = performance
        .exists()
        .then(|| read_json(&performance))
        .transpose()?;
    let budget_value = budgets.exists().then(|| read_json(&budgets)).transpose()?;
    let evaluator_violations = validate_evaluator(&evaluator);
    let manual_violations = validate_manual_qa(manual_value.as_ref());
    let budget_violations = validate_budgets(budget_value.as_ref(), &baseline_fingerprint);
    let performance_violations =
        validate_performance(performance_value.as_ref(), budget_value.as_ref());
    let mut violations = evaluator_violations
        .iter()
        .chain(manual_violations.iter())
        .chain(budget_violations.iter())
        .chain(performance_violations.iter())
        .cloned()
        .collect::<Vec<_>>();
    violations.sort();
    violations.dedup();
    let passed = violations.is_empty();
    let reviewed_live_count = evaluator
        .get("cases")
        .and_then(Value::as_array)
        .map(|cases| {
            cases
                .iter()
                .filter(|case| case.get("release_eligible").and_then(Value::as_bool) == Some(true))
                .count()
        })
        .unwrap_or(0);
    let locked_holdout_count = evaluator
        .get("cases")
        .and_then(Value::as_array)
        .map(|cases| {
            cases
                .iter()
                .filter(|case| {
                    case.get("release_eligible").and_then(Value::as_bool) == Some(true)
                        && case.get("partition").and_then(Value::as_str) == Some("locked_holdout")
                })
                .count()
        })
        .unwrap_or(0);
    let report = json!({
        "schema": FINAL_SCHEMA,
        "policy_version": POLICY_VERSION,
        "passed": passed,
        "authority_state": if passed { "authoritative" } else { "eligible" },
        "reviewed_live_count": reviewed_live_count,
        "locked_holdout_count": locked_holdout_count,
        "evaluator_release_gate_passed": evaluator.get("release_gate_passed"),
        "evaluator_validation_passed": evaluator_violations.is_empty(),
        "manual_macos_qa_passed": manual_violations.is_empty(),
        "manual_scenario_count": manual_value.as_ref().and_then(|value| value.get("scenarios")).and_then(Value::as_array).map(Vec::len).unwrap_or(0),
        "performance_cost_privacy_passed": performance_violations.is_empty(),
        "release_budget_policy_passed": budget_violations.is_empty(),
        "pre_holdout_baseline_report_id": baseline_fingerprint,
        "corpus_counts": evaluator.get("corpus_counts"),
        "partition_counts": evaluator.get("partition_counts"),
        "human_review_counts": evaluator.get("human_review_counts"),
        "surface_denominators": evaluator.get("reviewed_live_surface_denominators"),
        "slice_denominators": evaluator.get("reviewed_live_slice_denominators"),
        "macro_results": evaluator.get("reviewed_live_path_macro_results"),
        "worst_surface_family_slice": evaluator.get("worst_surface_family_slice_by_path"),
        "tt2_05_metric_results": evaluator.get("tt2_05_metric_results"),
        "tt2_05_surface_wrong_task_results": evaluator.get("tt2_05_surface_wrong_task_results"),
        "tt2_05_confidence_intervals": evaluator.get("tt2_05_confidence_intervals"),
        "zero_tolerance": {
            "control_navigation_as_task": evaluator.pointer("/tt2_05_metric_results/control_navigation_as_task_rate/numerator"),
            "stronger_manual_result_downgraded": evaluator.pointer("/tt2_05_metric_results/stronger_manual_result_downgraded/numerator"),
            "model_on_off_unexplained_task_disagreement": evaluator.pointer("/tt2_05_metric_results/model_on_off_unexplained_task_disagreement/numerator"),
            "privacy_violations": performance_value.as_ref().and_then(|value| value.get("privacy_violations")),
            "unsafe_opens": performance_value.as_ref().and_then(|value| value.get("unsafe_opens")),
            "background_multimodal_requests": performance_value.as_ref().and_then(|value| value.get("background_multimodal_requests")),
        },
        "validation": {
            "evaluator": evaluator_violations,
            "manual_macos_qa": manual_violations,
            "performance_cost_privacy": performance_violations,
            "release_budget_policy": budget_violations,
        },
        "evidence_inputs": {
            "task_truth_evaluator": input,
            "pre_holdout_baseline": baseline,
            "manual_macos_qa": manual_qa,
            "performance_cost_privacy": performance,
            "release_budget_policy": budgets,
        },
        "violations": violations,
    });
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(
        &output,
        serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passed_boolean_without_complete_evaluator_evidence_is_rejected() {
        let forged = json!({
            "schema": EVALUATOR_SCHEMA,
            "policy_version": POLICY_VERSION,
            "release_gate_passed": true,
            "release_gate_violations": []
        });
        let violations = validate_evaluator(&forged);
        assert!(violations
            .iter()
            .any(|item| item.contains("independently_reviewed_live")));
        assert!(violations
            .iter()
            .any(|item| item.contains("supported_next_action_coverage")));
    }

    #[test]
    fn passed_assessments_still_require_denominators_and_surface_intervals() {
        let forged = json!({
            "schema": EVALUATOR_SCHEMA,
            "policy_version": POLICY_VERSION,
            "release_gate_passed": true,
            "release_gate_violations": [],
            "tt2_05_metric_results": {
                "wrong_primary_task_rate": { "passed": true }
            },
            "tt2_05_confidence_intervals": {
                "wrong_primary_task_rate": {
                    "method": "wilson_score",
                    "lower": 0.0,
                    "upper": 0.01
                }
            },
            "tt2_05_surface_wrong_task_results": {
                "agent_chat": { "passed": true, "denominator": 15 }
            }
        });
        let violations = validate_evaluator(&forged);
        assert!(violations
            .iter()
            .any(|item| item == "tt2_05_metric_wrong_primary_task_rate_not_proven"));
        assert!(violations
            .iter()
            .any(|item| { item == "surface_agent_chat_wrong_task_confidence_interval_missing" }));
    }

    #[test]
    fn manual_manifest_requires_every_named_scenario_once() {
        let partial = json!({
            "schema": MANUAL_SCHEMA,
            "scenarios": [{
                "scenario_id": "privacy_blocking",
                "status": "passed",
                "reviewer": "reviewer-1",
                "app_build_commit": "commit-1",
                "expected_result": "Block transport",
                "actual_result": "Transport blocked",
                "evidence_ids": ["frame-redacted-1"]
            }]
        });
        let violations = validate_manual_qa(Some(&partial));
        assert!(violations
            .iter()
            .any(|item| item == "manual_scenario_not_right_feedback_scoped_missing"));
    }

    #[test]
    fn performance_passed_boolean_cannot_replace_measurements() {
        let forged = json!({
            "schema": PERFORMANCE_SCHEMA,
            "passed": true,
            "budget_policy_version": BUDGET_POLICY_VERSION,
            "policy_frozen_before_holdout_access": true,
            "holdout_accessed_at_freeze": false,
            "sample_count": 30,
            "background_multimodal_requests": 0,
            "privacy_violations": 0,
            "unsafe_opens": 0,
            "committed_fixture_secret_findings": 0,
            "privacy_blocked_frames_excluded_before_transport": true,
            "budgets": {},
            "measurements": {}
        });
        let budgets = json!({"budgets": {}});
        let violations = validate_performance(Some(&forged), Some(&budgets));
        assert!(violations
            .iter()
            .any(|item| item.contains("manual_continue_p95_ms")));
    }

    #[test]
    fn budget_policy_cannot_weaken_architectural_request_caps() {
        let policy = json!({
            "schema": BUDGET_SCHEMA,
            "policy_version": BUDGET_POLICY_VERSION,
            "baseline_report_id": "baseline-1",
            "frozen_before_holdout_access": true,
            "holdout_accessed_at_freeze": false,
            "budgets": {
                "manual_continue_p50_ms_max": 1000,
                "manual_continue_p95_ms_max": 3000,
                "local_packet_checkpoint_p50_ms_max": 50,
                "local_packet_checkpoint_p95_ms_max": 150,
                "model_request_max_images": 5,
                "model_request_max_bytes": 12582912,
                "model_request_max_tokens": 8000,
                "semantic_checkpoint_writes_per_minute_max": 12,
                "semantic_checkpoint_retention_count_max": 500,
                "provider_failure_rate_max": 0.05,
                "safe_fallback_rate_min": 0.99
            }
        });
        assert!(validate_budgets(Some(&policy), "sha256:expected")
            .iter()
            .any(|item| item == "release_budget_exceeds_architectural_privacy_or_retention_cap"));
    }

    #[test]
    fn budget_policy_must_bind_to_the_frozen_baseline_bytes() {
        let policy = json!({
            "schema": BUDGET_SCHEMA,
            "policy_version": BUDGET_POLICY_VERSION,
            "baseline_report_id": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "frozen_before_holdout_access": true,
            "holdout_accessed_at_freeze": false,
            "budgets": {}
        });
        assert!(validate_budgets(
            Some(&policy),
            "sha256:1111111111111111111111111111111111111111111111111111111111111111"
        )
        .iter()
        .any(|item| item == "release_budget_policy_not_frozen_from_baseline"));
    }
}
