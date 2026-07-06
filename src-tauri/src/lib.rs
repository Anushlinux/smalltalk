mod capture;
mod capture_core;
mod continuation;
mod session_island;

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
            capture::open_resume_point,
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
