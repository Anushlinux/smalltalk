mod capture;
mod capture_core;
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
            capture::start_memory,
            capture::pause_memory,
            capture::save_screenshot,
            capture::get_memory_status,
            capture::get_work_trail,
            capture::get_resume_card,
            capture::get_test_logs,
            capture::export_test_bundle,
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
