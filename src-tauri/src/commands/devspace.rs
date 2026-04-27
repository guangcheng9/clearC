use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevSpaceTarget {
    id: String,
    name: String,
    preferred_action: String,
    risk: String,
}

#[tauri::command]
pub fn get_devspace_targets() -> Vec<DevSpaceTarget> {
    vec![
        DevSpaceTarget {
            id: "cargo".to_string(),
            name: "Rust Cargo".to_string(),
            preferred_action: "env-migration".to_string(),
            risk: "medium".to_string(),
        },
        DevSpaceTarget {
            id: "aws".to_string(),
            name: "AWS 配置".to_string(),
            preferred_action: "readonly".to_string(),
            risk: "high".to_string(),
        },
    ]
}
