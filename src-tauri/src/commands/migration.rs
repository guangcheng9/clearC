use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationTarget {
    id: String,
    name: String,
    strategy: String,
    rollback: bool,
}

#[tauri::command]
pub fn get_migration_targets() -> Vec<MigrationTarget> {
    vec![
        MigrationTarget {
            id: "downloads".to_string(),
            name: "下载".to_string(),
            strategy: "shell-folder".to_string(),
            rollback: true,
        },
        MigrationTarget {
            id: "documents".to_string(),
            name: "文档".to_string(),
            strategy: "shell-folder".to_string(),
            rollback: true,
        },
    ]
}
