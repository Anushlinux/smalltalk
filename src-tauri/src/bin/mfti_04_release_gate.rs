use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const FINAL_SCHEMA: &str = "smalltalk.mfti_04.final_release_report.v1";
const POLICY_SCHEMA: &str = "smalltalk.mfti_04.eval_policy.v1";
const POLICY_VERSION: &str = "mfti.04-v1";
const EVALUATOR_SCHEMA: &str = "smalltalk.task_truth_v2.report.v1";
const IDENTITY_SCHEMA: &str = "smalltalk.mfti_04.release_identity.v1";
const MANUAL_SCHEMA: &str = "smalltalk.mfti_04.manual_macos_qa.v1";
const PERFORMANCE_SCHEMA: &str = "smalltalk.mfti_04.performance_cost_privacy.v1";

const REQUIRED_METRICS: [&str; 19] = [
    "wrong_primary_task_rate",
    "visible_surface_substituted_for_task",
    "wrong_activity_to_task_relationship",
    "wrong_task_switch_or_detour",
    "cross_session_stale_leakage",
    "mixed_snapshot_semantic_fields",
    "control_navigation_as_task_rate",
    "unsupported_specific_claim_rate",
    "provider_failure_local_semantic_fallback",
    "provider_failure_honest_unresolved",
    "useful_non_generic_task_summary",
    "task_object_accuracy",
    "execution_state_accuracy",
    "supported_next_action_precision",
    "supported_next_action_coverage",
    "return_target_precision",
    "stronger_manual_result_downgraded",
    "unseen_application_useful_summary",
    "human_immediately_useful",
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

const REQUIRED_MANUAL: [&str; 10] = [
    "smalltalk_work_then_related_ai_reading",
    "smalltalk_work_then_unrelated_ai_reading",
    "wallpaper_activity_after_engineering",
    "browser_tab_search_overlay",
    "provider_disabled_or_timeout",
    "old_snapshot_with_current_unresolved",
    "react_native_island_parity",
    "correct_task_without_openable_locator",
    "two_close_hypotheses_and_correction",
    "multiple_displays_and_other_window_ocr",
];

fn read_json(path: &Path) -> Result<Value, String> {
    serde_json::from_slice(&fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?)
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn nonempty(value: &Value, pointer: &str) -> bool {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .is_some_and(|v| !v.trim().is_empty())
}

fn validate_policy(value: &Value) -> Vec<String> {
    let mut violations = Vec::new();
    if value.pointer("/schema").and_then(Value::as_str) != Some(POLICY_SCHEMA)
        || value.pointer("/policy_version").and_then(Value::as_str) != Some(POLICY_VERSION)
    {
        violations.push("mfti_policy_schema_or_version_invalid".into());
    }
    if value
        .pointer("/frozen_before_holdout_access")
        .and_then(Value::as_bool)
        != Some(true)
        || value
            .pointer("/holdout_accessed_at_freeze")
            .and_then(Value::as_bool)
            != Some(false)
    {
        violations.push("mfti_policy_not_frozen_before_holdout".into());
    }
    for metric in REQUIRED_METRICS {
        let pointer = format!("/metrics/{metric}");
        let item = value.pointer(&pointer);
        if item
            .and_then(|v| v.get("threshold"))
            .and_then(Value::as_f64)
            .is_none()
            || item
                .and_then(|v| v.get("higher_is_better"))
                .and_then(Value::as_bool)
                .is_none()
            || item
                .and_then(|v| v.get("definition"))
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
        {
            violations.push(format!("mfti_policy_metric_{metric}_missing_or_invalid"));
        }
    }
    violations
}

fn validate_evaluator(value: &Value, policy: &Value) -> Vec<String> {
    let mut violations = Vec::new();
    if value.pointer("/schema").and_then(Value::as_str) != Some(EVALUATOR_SCHEMA) {
        violations.push("evaluator_schema_invalid".into());
    }
    let reviewed = value
        .pointer("/cases")
        .and_then(Value::as_array)
        .map(|cases| {
            cases
                .iter()
                .filter(|case| case.get("release_eligible").and_then(Value::as_bool) == Some(true))
                .count()
        })
        .unwrap_or(0);
    let holdout = value
        .pointer("/cases")
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
    if reviewed < 200 {
        violations.push(format!("reviewed_live_requires_200_found_{reviewed}"));
    }
    if holdout < 50 {
        violations.push(format!("locked_holdout_requires_50_found_{holdout}"));
    }

    for metric in REQUIRED_METRICS {
        let assessment = value.pointer(&format!("/mfti_04_metric_results/{metric}"));
        let denominator = assessment
            .and_then(|v| v.get("denominator"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let expected_threshold = policy
            .pointer(&format!("/metrics/{metric}/threshold"))
            .and_then(Value::as_f64);
        let expected_direction = policy
            .pointer(&format!("/metrics/{metric}/higher_is_better"))
            .and_then(Value::as_bool);
        if denominator == 0
            || assessment
                .and_then(|v| v.get("passed"))
                .and_then(Value::as_bool)
                != Some(true)
            || assessment
                .and_then(|v| v.get("threshold"))
                .and_then(Value::as_f64)
                != expected_threshold
            || assessment
                .and_then(|v| v.get("higher_is_better"))
                .and_then(Value::as_bool)
                != expected_direction
        {
            violations.push(format!("mfti_metric_{metric}_not_proven"));
        }
        let interval = value.pointer(&format!("/mfti_04_confidence_intervals/{metric}"));
        if interval
            .and_then(|v| v.get("denominator"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
            == 0
            || interval
                .and_then(|v| v.get("method"))
                .and_then(Value::as_str)
                != Some("wilson_score")
            || interval
                .and_then(|v| v.get("lower"))
                .and_then(Value::as_f64)
                .is_none()
            || interval
                .and_then(|v| v.get("upper"))
                .and_then(Value::as_f64)
                .is_none()
        {
            violations.push(format!("mfti_metric_{metric}_confidence_interval_missing"));
        }
    }
    if value
        .pointer("/mfti_04_metric_results/model_on_off_unexplained_task_disagreement")
        .is_some()
    {
        violations.push("model_on_off_disagreement_must_not_be_an_mfti_release_metric".into());
    }
    for surface in REQUIRED_SURFACES {
        let assessment = value.pointer(&format!("/mfti_04_surface_wrong_task_results/{surface}"));
        if assessment
            .and_then(|v| v.get("denominator"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
            < 15
            || assessment
                .and_then(|v| v.get("passed"))
                .and_then(Value::as_bool)
                != Some(true)
        {
            violations.push(format!("surface_{surface}_requires_15_reviewed_passes"));
        }
    }
    for (slice, minimum) in [
        ("interruption_resumption", 30_u64),
        ("ambiguous_or_privacy_blocked", 20),
        ("waiting_on_agent_or_application", 20),
        ("completed_vs_new_task", 20),
    ] {
        let found = value
            .pointer(&format!("/reviewed_live_slice_denominators/{slice}"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if found < minimum {
            violations.push(format!("slice_{slice}_requires_{minimum}_found_{found}"));
        }
    }
    violations
}

fn validate_identity(value: Option<&Value>) -> Vec<String> {
    let Some(value) = value else {
        return vec!["release_identity_manifest_missing".into()];
    };
    let mut violations = Vec::new();
    if value.pointer("/schema").and_then(Value::as_str) != Some(IDENTITY_SCHEMA) {
        violations.push("release_identity_schema_invalid".into());
    }
    for field in [
        "corpus_sha256",
        "holdout_sha256",
        "provider",
        "model",
        "prompt_version",
        "response_schema_version",
        "observation_packet_version",
        "verifier_version",
        "task_thread_version",
        "public_answer_version",
        "performance_privacy_policy_version",
        "manual_qa_manifest_sha256",
        "source_commit",
        "build_identity",
    ] {
        if !nonempty(value, &format!("/bindings/{field}")) {
            violations.push(format!("release_identity_{field}_missing"));
        }
    }
    violations
}

fn validate_manual(value: Option<&Value>) -> Vec<String> {
    let Some(value) = value else {
        return vec!["manual_macos_qa_manifest_missing".into()];
    };
    if value.pointer("/schema").and_then(Value::as_str) != Some(MANUAL_SCHEMA) {
        return vec!["manual_macos_qa_schema_invalid".into()];
    }
    let scenarios = value
        .pointer("/scenarios")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut seen = BTreeSet::new();
    let mut violations = Vec::new();
    for scenario in &scenarios {
        let id = scenario
            .get("scenario_id")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !seen.insert(id) {
            violations.push(format!("manual_scenario_{id}_duplicated"));
        }
        for field in [
            "reviewer",
            "app_build_commit",
            "expected_result",
            "actual_result",
            "provider",
            "model",
        ] {
            if scenario
                .get(field)
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
            {
                violations.push(format!("manual_scenario_{id}_{field}_missing"));
            }
        }
        if scenario.get("status").and_then(Value::as_str) != Some("passed")
            || scenario
                .get("evidence_ids")
                .and_then(Value::as_array)
                .is_none_or(Vec::is_empty)
        {
            violations.push(format!("manual_scenario_{id}_incomplete"));
        }
    }
    for required in REQUIRED_MANUAL {
        if !seen.contains(required) {
            violations.push(format!("manual_scenario_{required}_missing"));
        }
    }
    if scenarios.len() != REQUIRED_MANUAL.len() {
        violations.push(format!(
            "manual_scenario_count_expected_10_found_{}",
            scenarios.len()
        ));
    }
    violations
}

fn validate_performance(value: Option<&Value>) -> Vec<String> {
    let Some(value) = value else {
        return vec!["performance_cost_privacy_manifest_missing".into()];
    };
    let mut violations = Vec::new();
    if value.pointer("/schema").and_then(Value::as_str) != Some(PERFORMANCE_SCHEMA) {
        violations.push("performance_cost_privacy_schema_invalid".into());
    }
    if value
        .pointer("/policy_frozen_before_holdout_access")
        .and_then(Value::as_bool)
        != Some(true)
        || value
            .pointer("/holdout_accessed_at_freeze")
            .and_then(Value::as_bool)
            != Some(false)
    {
        violations.push("performance_policy_not_frozen_before_holdout".into());
    }
    if value
        .pointer("/sample_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        < 30
    {
        violations.push("performance_sample_count_requires_30".into());
    }
    for field in [
        "capture_to_packet_p50_ms",
        "capture_to_packet_p95_ms",
        "request_build_p50_ms",
        "request_build_p95_ms",
        "provider_p50_ms",
        "provider_p95_ms",
        "verification_persistence_p50_ms",
        "verification_persistence_p95_ms",
        "manual_continue_p50_ms",
        "manual_continue_p95_ms",
        "image_count_p50",
        "image_count_p95",
        "image_bytes_p50",
        "image_bytes_p95",
        "input_tokens_p50",
        "input_tokens_p95",
        "output_tokens_p50",
        "output_tokens_p95",
        "cost_per_continue_usd",
        "expected_monthly_cost_usd",
        "provider_timeout_rate",
        "provider_error_rate",
        "invalid_output_rate",
        "second_pass_rate",
        "second_pass_cost_usd",
        "privacy_exclusion_rate",
    ] {
        if value
            .pointer(&format!("/measurements/{field}"))
            .and_then(Value::as_f64)
            .is_none()
        {
            violations.push(format!("performance_measurement_{field}_missing"));
        }
    }
    for (field, expected) in [
        ("background_multimodal_requests", 0_u64),
        ("privacy_violations", 0),
        ("unsafe_opens", 0),
    ] {
        if value.get(field).and_then(Value::as_u64) != Some(expected) {
            violations.push(format!("performance_{field}_must_be_{expected}"));
        }
    }
    if value
        .get("privacy_blocked_frames_excluded_before_transport")
        .and_then(Value::as_bool)
        != Some(true)
        || value
            .get("provider_failure_experience_reviewed")
            .and_then(Value::as_bool)
            != Some(true)
    {
        violations.push("performance_privacy_or_failure_experience_unproven".into());
    }
    violations
}

fn main() {
    if let Err(error) = run() {
        eprintln!("MFTI-04 release gate failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let root = PathBuf::from("tests/fixtures/continue_accuracy/task_truth_v2/model_first");
    let mut evaluator = root.join("release-evaluator-report.v1.json");
    let mut policy = root.join("eval-policy.v1.json");
    let mut identity = root.join("release-identity.v1.json");
    let mut manual = root.join("manual-macos-qa.v1.json");
    let mut performance = root.join("performance-cost-privacy.v1.json");
    let mut output = root.join("final-release-report.v1.json");
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        let target = match arg.as_str() {
            "--evaluator" => &mut evaluator,
            "--policy" => &mut policy,
            "--identity" => &mut identity,
            "--manual-qa" => &mut manual,
            "--performance" => &mut performance,
            "--output" => &mut output,
            other => return Err(format!("unknown argument {other}")),
        };
        *target = PathBuf::from(args.next().ok_or(format!("{arg} requires a path"))?);
    }
    let evaluator_value = read_json(&evaluator)?;
    let policy_value = read_json(&policy)?;
    let identity_value = identity
        .exists()
        .then(|| read_json(&identity))
        .transpose()?;
    let manual_value = manual.exists().then(|| read_json(&manual)).transpose()?;
    let performance_value = performance
        .exists()
        .then(|| read_json(&performance))
        .transpose()?;
    let mut groups = BTreeMap::new();
    groups.insert("policy", validate_policy(&policy_value));
    groups.insert(
        "evaluator",
        validate_evaluator(&evaluator_value, &policy_value),
    );
    groups.insert(
        "release_identity",
        validate_identity(identity_value.as_ref()),
    );
    groups.insert("manual_macos_qa", validate_manual(manual_value.as_ref()));
    groups.insert(
        "performance_cost_privacy",
        validate_performance(performance_value.as_ref()),
    );
    let mut violations = groups.values().flatten().cloned().collect::<Vec<_>>();
    violations.sort();
    violations.dedup();
    let report = json!({
        "schema": FINAL_SCHEMA, "policy_version": POLICY_VERSION, "passed": violations.is_empty(),
        "authority_state": if violations.is_empty() { "authoritative" } else { "eligible" },
        "reviewed_live_count": evaluator_value.pointer("/cases").and_then(Value::as_array).map(|v| v.iter().filter(|c| c.get("release_eligible").and_then(Value::as_bool)==Some(true)).count()).unwrap_or(0),
        "locked_holdout_count": evaluator_value.pointer("/cases").and_then(Value::as_array).map(|v| v.iter().filter(|c| c.get("release_eligible").and_then(Value::as_bool)==Some(true) && c.get("partition").and_then(Value::as_str)==Some("locked_holdout")).count()).unwrap_or(0),
        "manual_scenario_count": manual_value.as_ref().and_then(|v| v.pointer("/scenarios")).and_then(Value::as_array).map(Vec::len).unwrap_or(0),
        "performance_sample_count": performance_value.as_ref().and_then(|v| v.pointer("/sample_count")).and_then(Value::as_u64).unwrap_or(0),
        "metric_results": evaluator_value.get("mfti_04_metric_results"),
        "surface_results": evaluator_value.get("mfti_04_surface_wrong_task_results"),
        "confidence_intervals": evaluator_value.get("mfti_04_confidence_intervals"),
        "bindings": identity_value.as_ref().and_then(|v| v.get("bindings")),
        "validation": groups, "violations": violations,
        "evidence_inputs": {"evaluator": evaluator, "policy": policy, "identity": identity, "manual_macos_qa": manual, "performance_cost_privacy": performance}
    });
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        &output,
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_denominator_cannot_pass() {
        let policy = read_json(Path::new(
            "tests/fixtures/continue_accuracy/task_truth_v2/model_first/eval-policy.v1.json",
        ))
        .unwrap();
        let evaluator = json!({"schema": EVALUATOR_SCHEMA, "cases": [], "mfti_04_metric_results": {}, "mfti_04_confidence_intervals": {}, "mfti_04_surface_wrong_task_results": {}, "reviewed_live_slice_denominators": {}});
        assert!(validate_evaluator(&evaluator, &policy)
            .iter()
            .any(|v| v.contains("not_proven")));
    }
    #[test]
    fn model_on_off_metric_cannot_replace_provider_failure_honesty() {
        let policy = read_json(Path::new(
            "tests/fixtures/continue_accuracy/task_truth_v2/model_first/eval-policy.v1.json",
        ))
        .unwrap();
        let evaluator = json!({"schema": EVALUATOR_SCHEMA, "cases": [], "mfti_04_metric_results": {"model_on_off_unexplained_task_disagreement":{"passed":true,"denominator":1}}, "mfti_04_confidence_intervals": {}, "mfti_04_surface_wrong_task_results": {}, "reviewed_live_slice_denominators": {}});
        let violations = validate_evaluator(&evaluator, &policy);
        assert!(violations
            .iter()
            .any(|v| v == "model_on_off_disagreement_must_not_be_an_mfti_release_metric"));
        assert!(violations
            .iter()
            .any(|v| v == "mfti_metric_provider_failure_honest_unresolved_not_proven"));
    }
    #[test]
    fn identity_manifest_requires_every_release_binding() {
        let violations = validate_identity(Some(
            &json!({"schema": IDENTITY_SCHEMA, "bindings": {"provider":"openai"}}),
        ));
        assert!(violations
            .iter()
            .any(|v| v == "release_identity_corpus_sha256_missing"));
        assert!(violations
            .iter()
            .any(|v| v == "release_identity_build_identity_missing"));
    }
}
