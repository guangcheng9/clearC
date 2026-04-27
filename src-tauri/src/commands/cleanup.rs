use crate::core::{
    paths::expand_path,
    rules::load_catalog,
    scan::{scan_rule_path, PathScanSummary},
};
use crate::storage::logs::{
    append_operation, data_dir_path, new_operation_id, now_ms, OperationLogEntry,
};
use serde::Serialize;
use serde_json::json;
use std::{fs, path::PathBuf};

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

#[tauri::command]
pub async fn get_cleanup_preview() -> Result<CleanupPreview, String> {
    tauri::async_runtime::spawn_blocking(build_cleanup_preview)
        .await
        .map_err(|err| format!("cleanup preview task failed: {}", err))?
}

#[tauri::command]
pub async fn create_cleanup_plan_draft() -> Result<CleanupPlanDraft, String> {
    tauri::async_runtime::spawn_blocking(create_cleanup_plan_draft_blocking)
        .await
        .map_err(|err| format!("cleanup plan task failed: {}", err))?
}

fn create_cleanup_plan_draft_blocking() -> Result<CleanupPlanDraft, String> {
    let preview = build_cleanup_preview()?;
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
    confirmation: String,
) -> Result<QuarantineCleanupResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        execute_temp_quarantine_cleanup_blocking(confirmation)
    })
    .await
    .map_err(|err| format!("quarantine cleanup task failed: {}", err))?
}

fn execute_temp_quarantine_cleanup_blocking(
    confirmation: String,
) -> Result<QuarantineCleanupResult, String> {
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

        for entry in entries {
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

            match fs::rename(&source, &destination) {
                Ok(()) => {
                    moved_count += 1;
                    moved_bytes = moved_bytes.saturating_add(size);
                    moved_items.push(MovedQuarantineItem {
                        original_path: source.display().to_string(),
                        quarantine_path: destination.display().to_string(),
                        bytes: size,
                    });
                }
                Err(err) => {
                    skipped_count += 1;
                    failures.push(CleanupFailure {
                        path: source.display().to_string(),
                        reason: err.to_string(),
                    });
                }
            }
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

fn build_cleanup_preview() -> Result<CleanupPreview, String> {
    let catalog = load_catalog()?;

    let items: Vec<CleanupPreviewItem> = catalog
        .cleanup
        .into_iter()
        .map(|rule| {
            let paths: Vec<PathScanSummary> =
                rule.paths.iter().map(|path| scan_rule_path(path)).collect();
            let estimated_bytes = paths.iter().map(|path| path.total_bytes).sum();
            let file_count = paths.iter().map(|path| path.file_count).sum();
            let skipped_count = paths.iter().map(|path| path.skipped_count).sum();

            CleanupPreviewItem {
                id: rule.id,
                name: rule.name,
                risk: rule.risk,
                action: rule.action,
                estimated_bytes,
                file_count,
                skipped_count,
                paths,
            }
        })
        .collect();

    let estimated_bytes = items.iter().map(|item| item.estimated_bytes).sum();
    let file_count = items.iter().map(|item| item.file_count).sum();
    let skipped_count = items.iter().map(|item| item.skipped_count).sum();

    Ok(CleanupPreview {
        estimated_bytes,
        file_count,
        skipped_count,
        requires_confirmation: true,
        executable: false,
        items,
    })
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
