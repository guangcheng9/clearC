mod commands;
mod core;
mod storage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::system::get_app_status,
            commands::scan::get_disk_overview,
            commands::scan::get_scan_targets,
            commands::scan::scan_cleanup_rules,
            commands::cleanup::create_cleanup_plan_draft,
            commands::cleanup::execute_temp_quarantine_cleanup,
            commands::cleanup::get_cleanup_preview,
            commands::rules::get_rule_catalog,
            commands::migration::get_migration_targets,
            commands::migration::get_user_folder_targets,
            commands::migration::precheck_user_folder_migration,
            commands::migration::create_user_folder_migration_plan,
            commands::migration::execute_user_folder_migration,
            commands::migration::rollback_user_folder_migration,
            commands::devspace::get_devspace_targets,
            commands::devspace::scan_devspace_targets,
            commands::logs::get_log_summary,
            commands::logs::rollback_quarantine_cleanup
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
