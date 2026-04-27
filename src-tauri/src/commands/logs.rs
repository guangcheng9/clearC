use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogSummary {
    total_operations: u64,
    rollbackable_operations: u64,
}

#[tauri::command]
pub fn get_log_summary() -> LogSummary {
    LogSummary {
        total_operations: 0,
        rollbackable_operations: 0,
    }
}
