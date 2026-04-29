use crate::commands::tasks::{clear_task_cancel, is_task_cancelled};
use crate::core::{
    paths::system_drive_root,
    rules::load_catalog,
    scan::{disk_space, scan_rule_path_with_progress, PathScanSummary, ScanProgressSnapshot},
};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgressEvent {
    task: String,
    current: u64,
    total: u64,
    label: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    scanned_files: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scanned_dirs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scanned_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_path: Option<String>,
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
pub async fn scan_cleanup_rules(app: AppHandle) -> Result<Vec<CleanupScanResult>, String> {
    tauri::async_runtime::spawn_blocking(move || scan_cleanup_rules_blocking(app))
        .await
        .map_err(|err| format!("scan task failed: {}", err))?
}

fn scan_cleanup_rules_blocking(app: AppHandle) -> Result<Vec<CleanupScanResult>, String> {
    clear_task_cancel("scan-cleanup-rules");
    let catalog = load_catalog()?;
    let total = catalog.cleanup.len() as u64;
    let mut results = Vec::new();

    emit_task_progress(&app, 0, total, "准备扫描".to_string(), "running");

    for (index, rule) in catalog.cleanup.into_iter().enumerate() {
        if is_task_cancelled("scan-cleanup-rules") {
            emit_task_progress(
                &app,
                index as u64,
                total,
                "扫描已取消".to_string(),
                "cancelled",
            );
            clear_task_cancel("scan-cleanup-rules");
            return Err("scan-cleanup-rules cancelled".to_string());
        }

        emit_task_progress(
            &app,
            index as u64,
            total,
            format!("正在扫描 {}", rule.name),
            "running",
        );

        let mut paths = Vec::new();
        for raw_path in &rule.paths {
            let result = scan_rule_path_with_progress(
                raw_path,
                &mut || is_task_cancelled("scan-cleanup-rules"),
                &mut |snapshot| {
                    emit_task_progress_with_scan(
                        &app,
                        index as u64,
                        total,
                        format!("正在扫描 {}", rule.name),
                        "running",
                        snapshot,
                    );
                },
            );

            match result {
                Ok(summary) => paths.push(summary),
                Err(summary) => {
                    emit_task_progress_with_scan(
                        &app,
                        index as u64,
                        total,
                        "扫描已取消".to_string(),
                        "cancelled",
                        ScanProgressSnapshot {
                            current_path: summary.resolved_path.clone(),
                            file_count: summary.file_count,
                            dir_count: summary.dir_count,
                            total_bytes: summary.total_bytes,
                            skipped_count: summary.skipped_count,
                        },
                    );
                    clear_task_cancel("scan-cleanup-rules");
                    return Err("scan-cleanup-rules cancelled".to_string());
                }
            }
        }
        let total_bytes = paths.iter().map(|path| path.total_bytes).sum();
        let file_count = paths.iter().map(|path| path.file_count).sum();
        let skipped_count = paths.iter().map(|path| path.skipped_count).sum();

        results.push(CleanupScanResult {
            id: rule.id,
            name: rule.name,
            risk: rule.risk,
            action: rule.action,
            paths,
            total_bytes,
            file_count,
            skipped_count,
        });

        emit_task_progress(
            &app,
            index as u64 + 1,
            total,
            "扫描规则完成".to_string(),
            "running",
        );
    }

    emit_task_progress(&app, total, total, "扫描完成".to_string(), "completed");
    clear_task_cancel("scan-cleanup-rules");

    Ok(results)
}

fn emit_task_progress(app: &AppHandle, current: u64, total: u64, label: String, status: &str) {
    emit_task_progress_payload(
        app, current, total, label, status, None, None, None, None, None,
    );
}

fn emit_task_progress_with_scan(
    app: &AppHandle,
    current: u64,
    total: u64,
    label: String,
    status: &str,
    snapshot: ScanProgressSnapshot,
) {
    emit_task_progress_payload(
        app,
        current,
        total,
        label,
        status,
        Some(snapshot.file_count),
        Some(snapshot.dir_count),
        Some(snapshot.total_bytes),
        Some(snapshot.skipped_count),
        Some(snapshot.current_path),
    );
}

fn emit_task_progress_payload(
    app: &AppHandle,
    current: u64,
    total: u64,
    label: String,
    status: &str,
    scanned_files: Option<u64>,
    scanned_dirs: Option<u64>,
    scanned_bytes: Option<u64>,
    skipped_count: Option<u64>,
    current_path: Option<String>,
) {
    let _ = app.emit(
        "clearc://task-progress",
        TaskProgressEvent {
            task: "scan-cleanup-rules".to_string(),
            current,
            total,
            label,
            status: status.to_string(),
            scanned_files,
            scanned_dirs,
            scanned_bytes,
            skipped_count,
            current_path,
        },
    );
}
