#![recursion_limit = "256"]

mod capture;
mod capture_core;
mod continuation;
mod session_island;
mod workload;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(capture::CaptureState::default())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            session_island::init_session_island(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            capture::start_capture,
            capture::get_screen_capture_permission_status,
            capture::request_screen_capture_permission,
            capture::stop_capture,
            capture::capture_once,
            capture::capture_status,
            capture::delete_all_frames,
            capture::get_local_memory_diagnostics,
            capture::cleanup_local_memory,
            capture::dev_reset_local_memory,
            capture::search_captures,
            capture::get_frame,
            capture::get_frame_image,
            capture::get_frame_image_variant,
            capture::start_native_capture,
            capture::stop_native_capture,
            capture::capture_once_v2,
            capture::get_frame_v2,
            capture::get_recent_timeline,
            capture::get_frame_detail,
            capture::validate_frame_consistency,
            capture::get_transition,
            capture::search_content_units,
            capture::build_safe_ai_export,
            capture::build_session_index,
            capture::build_resume_query_bundle,
            capture::run_cloud_resume,
            capture::get_cloud_resume_status,
            capture::get_continue_memory_status,
            capture::rebuild_continue_second_layer,
            capture::rebuild_continue_third_layer,
            capture::get_recent_continue_artifacts,
            capture::get_recent_continue_task_actions,
            capture::get_recent_continue_semantic_moments,
            capture::get_recent_continue_episodes,
            capture::get_recent_continue_workstreams,
            capture::get_continue_workstream_detail,
            capture::get_continue_decision,
            capture::get_continue_decision_trace,
            capture::add_continue_breadcrumb,
            capture::infer_continue_feedback,
            capture::record_continue_feedback,
            capture::run_continue_eval,
            capture::run_continue_replay_eval,
            capture::run_continue_accuracy_eval,
            capture::open_resume_point,
            session_island::get_island_continue_state,
            session_island::perform_island_continue_action,
            capture::get_native_storyboard_dossier,
            capture::classify_episode_transitions,
            capture::get_native_resume_card,
            capture::run_resume_eval,
            capture::export_debug_episode,
            capture::get_episode_dossier,
            capture::add_exclusion_rule,
            capture::remove_exclusion_rule,
            capture::list_exclusion_rules,
            capture::delete_recent_captures,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Public, side-effect-bounded entry point used by the repository accuracy CLI.
pub fn run_continue_accuracy_eval_cli(
    fixture_root: Option<String>,
    output_path: Option<String>,
    allow_locked_holdout: bool,
    repeat_count: usize,
) -> Result<serde_json::Value, String> {
    let report = continuation::accuracy_eval::run_committed_continue_accuracy_eval(
        continuation::accuracy_eval::ContinueAccuracyEvalOptions {
            fixture_root,
            allow_locked_holdout,
            repeat_count,
        },
    )?;
    if let Some(output_path) = output_path {
        continuation::accuracy_eval::write_accuracy_report(
            &report,
            std::path::Path::new(&output_path),
        )?;
    }
    serde_json::to_value(report).map_err(|error| error.to_string())
}

/// Local-only Task Truth v2.02 evaluator. This is deliberately separate from the P6 report.
pub fn run_task_truth_v2_eval_cli(
    fixture_root: Option<String>,
    output_path: Option<String>,
    allow_locked_holdout: bool,
) -> Result<serde_json::Value, String> {
    let root = fixture_root
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/continue_accuracy/task_truth_v2")
        });
    let report = continuation::task_truth_v2::evaluate(&root, allow_locked_holdout)?;
    if let Some(output_path) = output_path {
        continuation::task_truth_v2::write_report(&report, std::path::Path::new(&output_path))?;
    }
    serde_json::to_value(report).map_err(|error| error.to_string())
}

/// Stable content identity used to bind TT2-05 budgets to the frozen baseline bytes.
pub fn task_truth_v2_baseline_sha256(bytes: &[u8]) -> String {
    format!(
        "sha256:{}",
        continuation::accuracy_fixture::sha256_hex(bytes)
    )
}

/// Local-only, default-deny corpus builder used before any candidate is reviewed for commit.
pub fn build_task_truth_v2_candidate_cli(
    input_path: String,
    output_path: Option<String>,
    dry_run: bool,
) -> Result<serde_json::Value, String> {
    let manifest = continuation::task_truth_v2::build_review_candidate(
        std::path::Path::new(&input_path),
        output_path.as_deref().map(std::path::Path::new),
        dry_run,
    )?;
    serde_json::to_value(manifest).map_err(|error| error.to_string())
}
