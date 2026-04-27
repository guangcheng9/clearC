use crate::core::{
    paths::system_drive_root,
    rules::load_catalog,
    scan::{disk_space, scan_rule_path, PathScanSummary},
};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanTarget {
    id: String,
    name: String,
    risk: String,
    status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskOverview {
    drive: String,
    total_bytes: u64,
    free_bytes: u64,
    used_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupScanResult {
    id: String,
    name: String,
    risk: String,
    action: String,
    paths: Vec<PathScanSummary>,
    total_bytes: u64,
    file_count: u64,
    skipped_count: u64,
}

#[tauri::command]
pub fn get_scan_targets() -> Result<Vec<ScanTarget>, String> {
    let catalog = load_catalog()?;

    Ok(catalog
        .cleanup
        .into_iter()
        .map(|rule| ScanTarget {
            id: rule.id,
            name: rule.name,
            risk: rule.risk,
            status: "configured".to_string(),
        })
        .collect())
}

#[tauri::command]
pub fn get_disk_overview() -> Result<DiskOverview, String> {
    let drive = system_drive_root();
    let (total_bytes, free_bytes) = disk_space(&drive)?;

    Ok(DiskOverview {
        drive,
        total_bytes,
        free_bytes,
        used_bytes: total_bytes.saturating_sub(free_bytes),
    })
}

#[tauri::command]
pub async fn scan_cleanup_rules() -> Result<Vec<CleanupScanResult>, String> {
    tauri::async_runtime::spawn_blocking(scan_cleanup_rules_blocking)
        .await
        .map_err(|err| format!("scan task failed: {}", err))?
}

fn scan_cleanup_rules_blocking() -> Result<Vec<CleanupScanResult>, String> {
    let catalog = load_catalog()?;

    Ok(catalog
        .cleanup
        .into_iter()
        .map(|rule| {
            let paths: Vec<PathScanSummary> =
                rule.paths.iter().map(|path| scan_rule_path(path)).collect();
            let total_bytes = paths.iter().map(|path| path.total_bytes).sum();
            let file_count = paths.iter().map(|path| path.file_count).sum();
            let skipped_count = paths.iter().map(|path| path.skipped_count).sum();

            CleanupScanResult {
                id: rule.id,
                name: rule.name,
                risk: rule.risk,
                action: rule.action,
                paths,
                total_bytes,
                file_count,
                skipped_count,
            }
        })
        .collect())
}
