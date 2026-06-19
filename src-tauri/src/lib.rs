mod capture;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(capture::CaptureState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            capture::start_capture,
            capture::stop_capture,
            capture::capture_once,
            capture::capture_status,
            capture::search_captures,
            capture::get_frame,
            capture::get_frame_image,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
