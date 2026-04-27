use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanTarget {
    id: String,
    name: String,
    risk: String,
    status: String,
}

#[tauri::command]
pub fn get_scan_targets() -> Vec<ScanTarget> {
    vec![
        ScanTarget {
            id: "temp-user".to_string(),
            name: "用户临时文件".to_string(),
            risk: "safe".to_string(),
            status: "planned".to_string(),
        },
        ScanTarget {
            id: "recycle-bin".to_string(),
            name: "回收站".to_string(),
            risk: "safe".to_string(),
            status: "planned".to_string(),
        },
    ]
}
