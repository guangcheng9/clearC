use crate::core::{
    rules::load_catalog,
    scan::{scan_rule_path, PathScanSummary},
};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevSpaceTarget {
    id: String,
    name: String,
    preferred_action: String,
    risk: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevSpaceScanResult {
    id: String,
    name: String,
    risk: String,
    category: String,
    preferred_action: String,
    preferred_move: String,
    rollback: bool,
    exists: bool,
    total_bytes: u64,
    file_count: u64,
    skipped_count: u64,
    paths: Vec<PathScanSummary>,
}

#[tauri::command]
pub fn get_devspace_targets() -> Result<Vec<DevSpaceTarget>, String> {
    let catalog = load_catalog()?;

    Ok(catalog
        .devspace
        .into_iter()
        .map(|rule| DevSpaceTarget {
            id: rule.id,
            name: rule.name,
            preferred_action: rule.preferred_action,
            risk: rule.risk,
        })
        .collect())
}

#[tauri::command]
pub async fn scan_devspace_targets() -> Result<Vec<DevSpaceScanResult>, String> {
    tauri::async_runtime::spawn_blocking(scan_devspace_targets_blocking)
        .await
        .map_err(|err| format!("devspace scan task failed: {}", err))?
}

fn scan_devspace_targets_blocking() -> Result<Vec<DevSpaceScanResult>, String> {
    let catalog = load_catalog()?;

    Ok(catalog
        .devspace
        .into_iter()
        .map(|rule| {
            let paths: Vec<PathScanSummary> =
                rule.paths.iter().map(|path| scan_rule_path(path)).collect();
            let exists = paths.iter().any(|path| path.exists);
            let total_bytes = paths.iter().map(|path| path.total_bytes).sum();
            let file_count = paths.iter().map(|path| path.file_count).sum();
            let skipped_count = paths.iter().map(|path| path.skipped_count).sum();

            DevSpaceScanResult {
                id: rule.id,
                name: rule.name,
                risk: rule.risk,
                category: rule.category,
                preferred_action: rule.preferred_action,
                preferred_move: rule.preferred_move,
                rollback: rule.rollback,
                exists,
                total_bytes,
                file_count,
                skipped_count,
                paths,
            }
        })
        .collect())
}
