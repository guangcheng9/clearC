use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    app_name: String,
    version: String,
    native_core_ready: bool,
    rules_path: String,
}

#[tauri::command]
pub fn get_app_status() -> AppStatus {
    AppStatus {
        app_name: "ClearC".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        native_core_ready: true,
        rules_path: "rules".to_string(),
    }
}
