mod commands;
mod core;
mod storage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::system::get_app_status,
            commands::scan::get_scan_targets,
            commands::cleanup::get_cleanup_preview,
            commands::migration::get_migration_targets,
            commands::devspace::get_devspace_targets,
            commands::logs::get_log_summary
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
