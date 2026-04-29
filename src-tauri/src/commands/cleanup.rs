use crate::commands::tasks::{clear_task_cancel, is_task_cancelled};
use crate::core::{
    paths::expand_path,
    rules::load_catalog,
    scan::{scan_rule_path_with_progress, PathScanSummary},
};
use crate::storage::logs::{
    append_operation, data_dir_path, new_operation_id, now_ms, OperationLogEntry,
};
use serde::Serialize;
use serde_json::json;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Emitter};

const TEMP_USER_RULE_ID: &str = "temp-user";
const QUARANTINE_CONFIRMATION: &str = "MOVE_TO_QUARANTINE";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPreview {
    estimated_bytes: u64,
    file_count: u64,
    skipped_count: u64,
    requires_confirmation: bool,
    executable: bool,
    items: Vec<CleanupPreviewItem>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPreviewItem {
    id: String,
    name: String,
    risk: String,
    action: String,
    estimated_bytes: u64,
    file_count: u64,
    skipped_count: u64,
    paths: Vec<PathScanSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPlanDraft {
    id: String,
    status: String,
    executable: bool,
    estimated_bytes: u64,
    file_count: u64,
    skipped_count: u64,
    summary: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuarantineCleanupResult {
    id: String,
    status: String,
    moved_count: u64,
    skipped_count: u64,
    moved_bytes: u64,
    quarantine_path: String,
    failures: Vec<CleanupFailure>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupFailure {
    path: String,
    reason: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MovedQuarantineItem {
    original_path: String,
    quarantine_path: String,
    bytes: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupTaskProgressEvent {
    task: String,
    current: u64,
    total: u64,
    label: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    processed_items: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    moved_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    processed_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scanned_files: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scanned_dirs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scanned_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_file_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_file_total_bytes: Option<u64>,
}

#[tauri::command]
pub async fn get_cleanup_preview(app: AppHandle) -> Result<CleanupPreview, String> {
    tauri::async_runtime::spawn_blocking(move || build_cleanup_preview(app))
        .await
        .map_err(|err| format!("cleanup preview task failed: {}", err))?
}

#[tauri::command]
pub async fn create_cleanup_plan_draft(app: AppHandle) -> Result<CleanupPlanDraft, String> {
    tauri::async_runtime::spawn_blocking(move || create_cleanup_plan_draft_blocking(app))
        .await
        .map_err(|err| format!("cleanup plan task failed: {}", err))?
}

fn create_cleanup_plan_draft_blocking(app: AppHandle) -> Result<CleanupPlanDraft, String> {
    let preview = build_cleanup_preview(app)?;
    let id = new_operation_id("cleanup-plan");
    let summary = format!(
        "清理预览草稿：{} 个文件，预计释放 {} 字节",
        preview.file_count, preview.estimated_bytes
    );

    append_operation(&OperationLogEntry {
        id: id.clone(),
        operation_type: "cleanup-plan".to_string(),
        status: "planned".to_string(),
        created_at_ms: now_ms(),
        rollbackable: false,
        summary: summary.clone(),
        details: json!({
            "estimatedBytes": preview.estimated_bytes,
            "fileCount": preview.file_count,
            "skippedCount": preview.skipped_count,
            "requiresConfirmation": preview.requires_confirmation,
            "executable": preview.executable,
            "items": preview.items,
        }),
    })?;

    Ok(CleanupPlanDraft {
        id,
        status: "planned".to_string(),
        executable: false,
        estimated_bytes: preview.estimated_bytes,
        file_count: preview.file_count,
        skipped_count: preview.skipped_count,
        summary,
    })
}

#[tauri::command]
pub async fn execute_temp_quarantine_cleanup(
    app: AppHandle,
    confirmation: String,
) -> Result<QuarantineCleanupResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        execute_temp_quarantine_cleanup_blocking(app, confirmation)
    })
    .await
    .map_err(|err| format!("quarantine cleanup task failed: {}", err))?
}

fn execute_temp_quarantine_cleanup_blocking(
    app: AppHandle,
    confirmation: String,
) -> Result<QuarantineCleanupResult, String> {
    clear_task_cancel("cleanup-quarantine");
    if confirmation != QUARANTINE_CONFIRMATION {
        return Err("invalid cleanup confirmation".to_string());
    }

    let catalog = load_catalog()?;
    let rule = catalog
        .cleanup
        .into_iter()
        .find(|rule| rule.id == TEMP_USER_RULE_ID)
        .ok_or_else(|| "temp-user cleanup rule not found".to_string())?;

    if rule.risk != "safe" || rule.action != "delete" {
        return Err("temp-user rule is not eligible for quarantine cleanup".to_string());
    }

    let operation_id = new_operation_id("cleanup-quarantine");
    let quarantine_dir = data_dir_path()?.join("quarantine").join(&operation_id);
    fs::create_dir_all(&quarantine_dir).map_err(|err| {
        format!(
            "failed to create quarantine dir {}: {}",
            quarantine_dir.display(),
            err
        )
    })?;

    let mut moved_count = 0u64;
    let mut skipped_count = 0u64;
    let mut moved_bytes = 0u64;
    let mut failures = Vec::new();
    let mut moved_items = Vec::new();
    let exclude = rule.exclude;

    emit_cleanup_task_progress(
        &app,
        "cleanup-quarantine",
        0,
        1,
        "准备隔离清理".to_string(),
        "running",
    );

    for raw_path in rule.paths {
        let source_dir = expand_path(&raw_path);
        let entries = match fs::read_dir(&source_dir) {
            Ok(entries) => entries,
            Err(err) => {
                failures.push(CleanupFailure {
                    path: source_dir.display().to_string(),
                    reason: err.to_string(),
                });
                skipped_count += 1;
                continue;
            }
        };
        let entries: Vec<_> = entries.collect();
        let total = entries.len() as u64;

        for (index, entry) in entries.into_iter().enumerate() {
            if is_task_cancelled("cleanup-quarantine") {
                emit_cleanup_task_progress(
                    &app,
                    "cleanup-quarantine",
                    index as u64,
                    total,
                    "隔离清理已取消".to_string(),
                    "cancelled",
                );
                clear_task_cancel("cleanup-quarantine");
                return Err("cleanup-quarantine cancelled".to_string());
            }

            emit_cleanup_task_progress_with_stats(
                &app,
                "cleanup-quarantine",
                index as u64,
                total,
                format!("正在隔离 {}", source_dir.display()),
                "running",
                CleanupProgressStats {
                    processed_items: Some(moved_count.saturating_add(skipped_count)),
                    moved_count: Some(moved_count),
                    skipped_count: Some(skipped_count),
                    processed_bytes: Some(moved_bytes),
                    failure_count: Some(failures.len() as u64),
                    ..CleanupProgressStats::default()
                },
            );
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    failures.push(CleanupFailure {
                        path: source_dir.display().to_string(),
                        reason: err.to_string(),
                    });
                    skipped_count += 1;
                    continue;
                }
            };

            let source = entry.path();
            let file_name = match source.file_name() {
                Some(file_name) => file_name.to_string_lossy().to_string(),
                None => {
                    skipped_count += 1;
                    continue;
                }
            };

            if is_excluded(&file_name, &exclude) {
                skipped_count += 1;
                continue;
            }

            let size = path_size(&source).unwrap_or(0);
            let destination = unique_destination(&quarantine_dir, &file_name);

            let move_progress = CleanupMoveProgressContext {
                app: &app,
                item_index: index as u64,
                total_items: total,
                moved_count,
                skipped_count,
                moved_bytes,
                failure_count: failures.len() as u64,
            };
            match move_path_to_quarantine(&source, &destination, size, &move_progress) {
                Ok(move_bytes) => {
                    moved_count += 1;
                    moved_bytes = moved_bytes.saturating_add(move_bytes);
                    moved_items.push(MovedQuarantineItem {
                        original_path: source.display().to_string(),
                        quarantine_path: destination.display().to_string(),
                        bytes: move_bytes,
                    });
                }
                Err(err) => {
                    if err.contains("cancelled") {
                        emit_cleanup_task_progress(
                            &app,
                            "cleanup-quarantine",
                            index as u64,
                            total,
                            "隔离清理已取消".to_string(),
                            "cancelled",
                        );
                        clear_task_cancel("cleanup-quarantine");
                        return Err(err);
                    }
                    skipped_count += 1;
                    failures.push(CleanupFailure {
                        path: source.display().to_string(),
                        reason: err.to_string(),
                    });
                }
            }

            emit_cleanup_task_progress_with_stats(
                &app,
                "cleanup-quarantine",
                index as u64 + 1,
                total,
                format!("已处理 {}", source.display()),
                "running",
                CleanupProgressStats {
                    processed_items: Some(moved_count.saturating_add(skipped_count)),
                    moved_count: Some(moved_count),
                    skipped_count: Some(skipped_count),
                    processed_bytes: Some(moved_bytes),
                    failure_count: Some(failures.len() as u64),
                    ..CleanupProgressStats::default()
                },
            );
        }
    }

    let status = if failures.is_empty() {
        "completed"
    } else {
        "completed-with-skips"
    }
    .to_string();
    let summary = format!(
        "隔离清理完成：移动 {} 项，跳过 {} 项，隔离 {} 字节",
        moved_count, skipped_count, moved_bytes
    );

    append_operation(&OperationLogEntry {
        id: operation_id.clone(),
        operation_type: "cleanup-quarantine".to_string(),
        status: status.clone(),
        created_at_ms: now_ms(),
        rollbackable: moved_count > 0,
        summary,
        details: json!({
            "ruleId": TEMP_USER_RULE_ID,
            "movedCount": moved_count,
            "skippedCount": skipped_count,
            "movedBytes": moved_bytes,
            "quarantinePath": quarantine_dir,
            "movedItems": moved_items,
            "failures": failures,
        }),
    })?;

    emit_cleanup_task_progress_with_stats(
        &app,
        "cleanup-quarantine",
        moved_count.saturating_add(skipped_count),
        moved_count.saturating_add(skipped_count),
        "隔离清理完成".to_string(),
        "completed",
        CleanupProgressStats {
            processed_items: Some(moved_count.saturating_add(skipped_count)),
            moved_count: Some(moved_count),
            skipped_count: Some(skipped_count),
            processed_bytes: Some(moved_bytes),
            failure_count: Some(failures.len() as u64),
            ..CleanupProgressStats::default()
        },
    );
    clear_task_cancel("cleanup-quarantine");

    Ok(QuarantineCleanupResult {
        id: operation_id,
        status,
        moved_count,
        skipped_count,
        moved_bytes,
        quarantine_path: quarantine_dir.display().to_string(),
        failures,
    })
}

fn build_cleanup_preview(app: AppHandle) -> Result<CleanupPreview, String> {
    clear_task_cancel("cleanup-preview");
    let catalog = load_catalog()?;
    let total = catalog.cleanup.len() as u64;
    emit_cleanup_task_progress(
        &app,
        "cleanup-preview",
        0,
        total,
        "准备生成清理预览".to_string(),
        "running",
    );

    let items: Vec<CleanupPreviewItem> = catalog
        .cleanup
        .into_iter()
        .enumerate()
        .map(|(index, rule)| {
            if is_task_cancelled("cleanup-preview") {
                emit_cleanup_task_progress(
                    &app,
                    "cleanup-preview",
                    index as u64,
                    total,
                    "清理预览已取消".to_string(),
                    "cancelled",
                );
                clear_task_cancel("cleanup-preview");
                return Err("cleanup-preview cancelled".to_string());
            }

            emit_cleanup_task_progress(
                &app,
                "cleanup-preview",
                index as u64,
                total,
                format!("正在扫描 {}", rule.name),
                "running",
            );
            let mut paths = Vec::new();
            for raw_path in &rule.paths {
                let result = scan_rule_path_with_progress(
                    raw_path,
                    &mut || is_task_cancelled("cleanup-preview"),
                    &mut |snapshot| {
                        emit_cleanup_task_progress_with_stats(
                            &app,
                            "cleanup-preview",
                            index as u64,
                            total,
                            format!("正在扫描 {}", rule.name),
                            "running",
                            CleanupProgressStats {
                                scanned_files: Some(snapshot.file_count),
                                scanned_dirs: Some(snapshot.dir_count),
                                scanned_bytes: Some(snapshot.total_bytes),
                                skipped_count: Some(snapshot.skipped_count),
                                current_path: Some(snapshot.current_path),
                                ..CleanupProgressStats::default()
                            },
                        );
                    },
                );

                match result {
                    Ok(summary) => paths.push(summary),
                    Err(summary) => {
                        emit_cleanup_task_progress_with_stats(
                            &app,
                            "cleanup-preview",
                            index as u64,
                            total,
                            "清理预览已取消".to_string(),
                            "cancelled",
                            CleanupProgressStats {
                                scanned_files: Some(summary.file_count),
                                scanned_dirs: Some(summary.dir_count),
                                scanned_bytes: Some(summary.total_bytes),
                                skipped_count: Some(summary.skipped_count),
                                current_path: Some(summary.resolved_path),
                                ..CleanupProgressStats::default()
                            },
                        );
                        clear_task_cancel("cleanup-preview");
                        return Err("cleanup-preview cancelled".to_string());
                    }
                }
            }
            let estimated_bytes = paths.iter().map(|path| path.total_bytes).sum();
            let file_count = paths.iter().map(|path| path.file_count).sum();
            let skipped_count = paths.iter().map(|path| path.skipped_count).sum();

            Ok(CleanupPreviewItem {
                id: rule.id,
                name: rule.name,
                risk: rule.risk,
                action: rule.action,
                estimated_bytes,
                file_count,
                skipped_count,
                paths,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let estimated_bytes = items.iter().map(|item| item.estimated_bytes).sum();
    let file_count = items.iter().map(|item| item.file_count).sum();
    let skipped_count = items.iter().map(|item| item.skipped_count).sum();

    emit_cleanup_task_progress(
        &app,
        "cleanup-preview",
        total,
        total,
        "清理预览完成".to_string(),
        "completed",
    );
    clear_task_cancel("cleanup-preview");

    Ok(CleanupPreview {
        estimated_bytes,
        file_count,
        skipped_count,
        requires_confirmation: true,
        executable: false,
        items,
    })
}

fn emit_cleanup_task_progress(
    app: &AppHandle,
    task: &str,
    current: u64,
    total: u64,
    label: String,
    status: &str,
) {
    emit_cleanup_task_progress_with_stats(
        app,
        task,
        current,
        total,
        label,
        status,
        CleanupProgressStats::default(),
    );
}

#[derive(Default)]
struct CleanupProgressStats {
    processed_items: Option<u64>,
    moved_count: Option<u64>,
    skipped_count: Option<u64>,
    processed_bytes: Option<u64>,
    failure_count: Option<u64>,
    scanned_files: Option<u64>,
    scanned_dirs: Option<u64>,
    scanned_bytes: Option<u64>,
    current_path: Option<String>,
    current_file_path: Option<String>,
    current_file_bytes: Option<u64>,
    current_file_total_bytes: Option<u64>,
}

fn emit_cleanup_task_progress_with_stats(
    app: &AppHandle,
    task: &str,
    current: u64,
    total: u64,
    label: String,
    status: &str,
    stats: CleanupProgressStats,
) {
    let _ = app.emit(
        "clearc://task-progress",
        CleanupTaskProgressEvent {
            task: task.to_string(),
            current,
            total,
            label,
            status: status.to_string(),
            processed_items: stats.processed_items,
            moved_count: stats.moved_count,
            skipped_count: stats.skipped_count,
            processed_bytes: stats.processed_bytes,
            failure_count: stats.failure_count,
            scanned_files: stats.scanned_files,
            scanned_dirs: stats.scanned_dirs,
            scanned_bytes: stats.scanned_bytes,
            current_path: stats.current_path,
            current_file_path: stats.current_file_path,
            current_file_bytes: stats.current_file_bytes,
            current_file_total_bytes: stats.current_file_total_bytes,
        },
    );
}

fn is_excluded(file_name: &str, exclude: &[String]) -> bool {
    exclude.iter().any(|pattern| {
        if let Some(suffix) = pattern.strip_prefix("*.") {
            file_name.ends_with(&format!(".{}", suffix))
        } else {
            file_name == pattern
        }
    })
}

fn path_size(path: &PathBuf) -> Result<u64, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|err| format!("failed to read {}: {}", path.display(), err))?;

    if metadata.is_file() {
        return Ok(metadata.len());
    }

    if !metadata.is_dir() {
        return Ok(0);
    }

    let mut total = 0u64;
    for entry in fs::read_dir(path)
        .map_err(|err| format!("failed to read dir {}: {}", path.display(), err))?
    {
        let entry = entry.map_err(|err| err.to_string())?;
        total = total.saturating_add(path_size(&entry.path()).unwrap_or(0));
    }

    Ok(total)
}

struct CleanupMoveProgressContext<'a> {
    app: &'a AppHandle,
    item_index: u64,
    total_items: u64,
    moved_count: u64,
    skipped_count: u64,
    moved_bytes: u64,
    failure_count: u64,
}

fn move_path_to_quarantine(
    source: &Path,
    destination: &Path,
    expected_bytes: u64,
    progress: &CleanupMoveProgressContext,
) -> Result<u64, String> {
    if destination.exists() {
        return Err("quarantine target already exists".to_string());
    }

    match fs::rename(source, destination) {
        Ok(()) => return Ok(expected_bytes),
        Err(rename_err) => {
            let metadata = fs::symlink_metadata(source)
                .map_err(|err| format!("failed to read {}: {}", source.display(), err))?;
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "symlink quarantine is not supported after rename failed: {}",
                    rename_err
                ));
            }
        }
    }

    if let Err(err) = copy_path_to_quarantine(source, destination, progress) {
        let _ = remove_path(destination);
        return Err(err);
    }

    let copied_bytes = path_size(&destination.to_path_buf()).unwrap_or(0);
    if copied_bytes != expected_bytes {
        let _ = remove_path(destination);
        return Err(format!(
            "copied size mismatch: expected {} bytes, copied {} bytes",
            expected_bytes, copied_bytes
        ));
    }

    if let Err(err) = remove_path(source) {
        let cleanup_result = remove_path(destination);
        return Err(match cleanup_result {
            Ok(()) => format!("failed to remove source after copy: {}", err),
            Err(cleanup_err) => format!(
                "failed to remove source after copy: {}; copied target cleanup also failed: {}",
                err, cleanup_err
            ),
        });
    }

    Ok(copied_bytes)
}

fn copy_path_to_quarantine(
    source: &Path,
    destination: &Path,
    progress: &CleanupMoveProgressContext,
) -> Result<(), String> {
    check_quarantine_cancel()?;
    let metadata = fs::symlink_metadata(source)
        .map_err(|err| format!("failed to read {}: {}", source.display(), err))?;

    if metadata.is_file() {
        return copy_file_to_quarantine(source, destination, progress);
    }

    if !metadata.is_dir() {
        return Err("unsupported file type".to_string());
    }

    fs::create_dir_all(destination)
        .map_err(|err| format!("failed to create {}: {}", destination.display(), err))?;

    for entry in fs::read_dir(source)
        .map_err(|err| format!("failed to read dir {}: {}", source.display(), err))?
    {
        let entry = entry.map_err(|err| err.to_string())?;
        let source_child = entry.path();
        let destination_child = destination.join(entry.file_name());
        if destination_child.exists() {
            return Err(format!(
                "quarantine target already exists: {}",
                destination_child.display()
            ));
        }
        copy_path_to_quarantine(&source_child, &destination_child, progress)?;
    }

    Ok(())
}

fn copy_file_to_quarantine(
    source: &Path,
    destination: &Path,
    progress: &CleanupMoveProgressContext,
) -> Result<(), String> {
    check_quarantine_cancel()?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {}", parent.display(), err))?;
    }

    let total_bytes = fs::symlink_metadata(source)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let mut copied_bytes = 0u64;
    let mut reader = fs::File::open(source)
        .map_err(|err| format!("failed to open {}: {}", source.display(), err))?;
    let mut writer = fs::File::create(destination)
        .map_err(|err| format!("failed to create {}: {}", destination.display(), err))?;
    let mut buffer = vec![0u8; 1024 * 1024];

    loop {
        check_quarantine_cancel()?;
        let bytes_read = reader
            .read(&mut buffer)
            .map_err(|err| format!("failed to read {}: {}", source.display(), err))?;
        if bytes_read == 0 {
            break;
        }
        writer
            .write_all(&buffer[..bytes_read])
            .map_err(|err| format!("failed to write {}: {}", destination.display(), err))?;
        copied_bytes = copied_bytes.saturating_add(bytes_read as u64);
        emit_quarantine_copy_progress(progress, source, copied_bytes, total_bytes);
    }

    writer
        .flush()
        .map_err(|err| format!("failed to flush {}: {}", destination.display(), err))?;
    Ok(())
}

fn emit_quarantine_copy_progress(
    progress: &CleanupMoveProgressContext,
    source: &Path,
    copied_bytes: u64,
    total_bytes: u64,
) {
    emit_cleanup_task_progress_with_stats(
        progress.app,
        "cleanup-quarantine",
        progress.item_index,
        progress.total_items,
        format!("正在隔离 {}", source.display()),
        "running",
        CleanupProgressStats {
            processed_items: Some(progress.moved_count.saturating_add(progress.skipped_count)),
            moved_count: Some(progress.moved_count),
            skipped_count: Some(progress.skipped_count),
            processed_bytes: Some(progress.moved_bytes.saturating_add(copied_bytes)),
            failure_count: Some(progress.failure_count),
            current_file_path: Some(source.display().to_string()),
            current_file_bytes: Some(copied_bytes),
            current_file_total_bytes: Some(total_bytes),
            ..CleanupProgressStats::default()
        },
    );
}

fn check_quarantine_cancel() -> Result<(), String> {
    if is_task_cancelled("cleanup-quarantine") {
        return Err("cleanup-quarantine cancelled".to_string());
    }
    Ok(())
}

fn remove_path(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|err| format!("failed to read {}: {}", path.display(), err))?;
    if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
    .map_err(|err| format!("failed to remove {}: {}", path.display(), err))
}

fn unique_destination(base_dir: &PathBuf, file_name: &str) -> PathBuf {
    let mut destination = base_dir.join(file_name);
    if !destination.exists() {
        return destination;
    }

    for index in 1.. {
        destination = base_dir.join(format!("{}-{}", index, file_name));
        if !destination.exists() {
            return destination;
        }
    }

    destination
}
