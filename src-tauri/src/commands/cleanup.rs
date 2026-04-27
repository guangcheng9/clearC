use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPreview {
    estimated_bytes: u64,
    file_count: u64,
    requires_confirmation: bool,
}

#[tauri::command]
pub fn get_cleanup_preview() -> CleanupPreview {
    CleanupPreview {
        estimated_bytes: 0,
        file_count: 0,
        requires_confirmation: true,
    }
}
