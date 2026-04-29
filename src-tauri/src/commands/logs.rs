use crate::storage::logs::{
    append_operation, data_dir_path, new_operation_id, now_ms, read_operations, OperationLogEntry,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

const ROLLBACK_CONFIRMATION: &str = "RESTORE_FROM_QUARANTINE";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogSummary {
    total_operations: u64,
    rollbackable_operations: u64,
    planned_operations: u64,
    failed_operations: u64,
    recent: Vec<LogEntryView>,
}

#[tauri::command]
pub fn get_log_summary() -> Result<LogSummary, String> {
    let mut entries = read_operations()?;
    let rollback_targets = rollback_targets(&entries);
    entries.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));

    let total_operations = entries.len() as u64;
    let rollbackable_operations = entries.iter().filter(|entry| entry.rollbackable).count() as u64;
    let planned_operations = entries
        .iter()
        .filter(|entry| entry.status == "planned")
        .count() as u64;
    let failed_operations = entries
        .iter()
        .filter(|entry| entry.status.contains("failure") || entry.status.contains("skips"))
        .count() as u64;
    let recent = entries
        .into_iter()
        .take(20)
        .map(|entry| LogEntryView {
            failure_count: extract_failures(&entry.details).len() as u64,
            can_rollback: entry.rollbackable && !rollback_targets.contains(&entry.id),
            id: entry.id,
            operation_type: entry.operation_type,
            status: entry.status,
            created_at_ms: entry.created_at_ms,
            rollbackable: entry.rollbackable,
            summary: entry.summary,
        })
        .collect();

    Ok(LogSummary {
        total_operations,
        rollbackable_operations,
        planned_operations,
        failed_operations,
        recent,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntryView {
    id: String,
    operation_type: String,
    status: String,
    created_at_ms: u128,
    rollbackable: bool,
    can_rollback: bool,
    summary: String,
    failure_count: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackResult {
    id: String,
    rollback_of: String,
    status: String,
    restored_count: u64,
    skipped_count: u64,
    failures: Vec<RollbackFailure>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackFailure {
    path: String,
    reason: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailureExportResult {
    operation_id: String,
    exported_count: u64,
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MovedItem {
    original_path: String,
    quarantine_path: String,
}

#[tauri::command]
pub async fn rollback_quarantine_cleanup(
    operation_id: String,
    confirmation: String,
) -> Result<RollbackResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        rollback_quarantine_cleanup_blocking(operation_id, confirmation)
    })
    .await
    .map_err(|err| format!("rollback task failed: {}", err))?
}

#[tauri::command]
pub async fn export_operation_failures(
    operation_id: String,
) -> Result<FailureExportResult, String> {
    tauri::async_runtime::spawn_blocking(move || export_operation_failures_blocking(operation_id))
        .await
        .map_err(|err| format!("failure export task failed: {}", err))?
}

#[tauri::command]
pub async fn open_failure_export_folder() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(open_failure_export_folder_blocking)
        .await
        .map_err(|err| format!("open export folder task failed: {}", err))?
}

fn open_failure_export_folder_blocking() -> Result<String, String> {
    let export_dir = data_dir_path()?.join("exports");
    fs::create_dir_all(&export_dir).map_err(|err| {
        format!(
            "failed to create export dir {}: {}",
            export_dir.display(),
            err
        )
    })?;
    tauri_plugin_opener::open_path(&export_dir, None::<&str>)
        .map_err(|err| format!("failed to open {}: {}", export_dir.display(), err))?;
    Ok(export_dir.display().to_string())
}

fn export_operation_failures_blocking(operation_id: String) -> Result<FailureExportResult, String> {
    let entries = read_operations()?;
    let entry = entries
        .iter()
        .find(|entry| entry.id == operation_id)
        .ok_or_else(|| format!("operation {} not found", operation_id))?;
    let failures = extract_failures(&entry.details);

    if failures.is_empty() {
        return Err("operation has no failure items to export".to_string());
    }

    let export_dir = data_dir_path()?.join("exports");
    fs::create_dir_all(&export_dir).map_err(|err| {
        format!(
            "failed to create export dir {}: {}",
            export_dir.display(),
            err
        )
    })?;
    let file_path = export_dir.join(format!("failures-{}.csv", safe_file_name(&operation_id)));
    let mut content = String::from("operationId,operationType,status,path,reason\n");
    for failure in &failures {
        content.push_str(&format!(
            "{},{},{},{},{}\n",
            csv_escape(&entry.id),
            csv_escape(&entry.operation_type),
            csv_escape(&entry.status),
            csv_escape(&failure.path),
            csv_escape(&failure.reason)
        ));
    }

    fs::write(&file_path, content)
        .map_err(|err| format!("failed to write {}: {}", file_path.display(), err))?;

    Ok(FailureExportResult {
        operation_id,
        exported_count: failures.len() as u64,
        path: file_path.display().to_string(),
    })
}

fn rollback_quarantine_cleanup_blocking(
    operation_id: String,
    confirmation: String,
) -> Result<RollbackResult, String> {
    if confirmation != ROLLBACK_CONFIRMATION {
        return Err("invalid rollback confirmation".to_string());
    }

    let entries = read_operations()?;
    let rollback_targets = rollback_targets(&entries);
    if rollback_targets.contains(&operation_id) {
        return Err("operation already has a rollback record".to_string());
    }

    let entry = entries
        .iter()
        .find(|entry| entry.id == operation_id)
        .ok_or_else(|| format!("operation {} not found", operation_id))?;

    if entry.operation_type != "cleanup-quarantine" || !entry.rollbackable {
        return Err("operation is not rollbackable".to_string());
    }

    let moved_items: Vec<MovedItem> = serde_json::from_value(
        entry
            .details
            .get("movedItems")
            .cloned()
            .ok_or_else(|| "operation log does not include movedItems".to_string())?,
    )
    .map_err(|err| format!("failed to parse movedItems: {}", err))?;

    let mut restored_count = 0u64;
    let mut skipped_count = 0u64;
    let mut failures = Vec::new();

    for item in moved_items {
        let source = PathBuf::from(&item.quarantine_path);
        let destination = PathBuf::from(&item.original_path);

        if !source.exists() {
            skipped_count += 1;
            failures.push(RollbackFailure {
                path: source.display().to_string(),
                reason: "quarantine item does not exist".to_string(),
            });
            continue;
        }

        if destination.exists() {
            skipped_count += 1;
            failures.push(RollbackFailure {
                path: destination.display().to_string(),
                reason: "original path already exists".to_string(),
            });
            continue;
        }

        if let Some(parent) = destination.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                skipped_count += 1;
                failures.push(RollbackFailure {
                    path: parent.display().to_string(),
                    reason: err.to_string(),
                });
                continue;
            }
        }

        match fs::rename(&source, &destination) {
            Ok(()) => restored_count += 1,
            Err(err) => {
                skipped_count += 1;
                failures.push(RollbackFailure {
                    path: source.display().to_string(),
                    reason: err.to_string(),
                });
            }
        }
    }

    let status = if failures.is_empty() {
        "rolled-back"
    } else {
        "rolled-back-with-skips"
    }
    .to_string();
    let rollback_id = new_operation_id("rollback");
    let summary = format!(
        "隔离清理回滚：恢复 {} 项，跳过 {} 项",
        restored_count, skipped_count
    );

    append_operation(&OperationLogEntry {
        id: rollback_id.clone(),
        operation_type: "rollback".to_string(),
        status: status.clone(),
        created_at_ms: now_ms(),
        rollbackable: false,
        summary,
        details: json!({
            "rollbackOf": operation_id,
            "restoredCount": restored_count,
            "skippedCount": skipped_count,
            "failures": failures,
        }),
    })?;

    Ok(RollbackResult {
        id: rollback_id,
        rollback_of: operation_id,
        status,
        restored_count,
        skipped_count,
        failures,
    })
}

fn rollback_targets(entries: &[OperationLogEntry]) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| {
            entry.operation_type == "rollback"
                || entry.operation_type == "user-folder-migration-rollback"
                || entry.operation_type == "devspace-env-migration-rollback"
                || entry.operation_type == "devspace-junction-migration-rollback"
        })
        .filter_map(|entry| {
            entry
                .details
                .get("rollbackOf")
                .and_then(|value| value.as_str())
                .map(ToString::to_string)
        })
        .collect()
}

fn extract_failures(details: &Value) -> Vec<RollbackFailure> {
    details
        .get("failures")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .map(|item| RollbackFailure {
                    path: item
                        .get("path")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string(),
                    reason: item
                        .get("reason")
                        .and_then(|value| value.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn safe_file_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ => ch,
        })
        .collect()
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{csv_escape, extract_failures, safe_file_name};
    use serde_json::json;

    #[test]
    fn extracts_failures_from_details() {
        let failures = extract_failures(&json!({
            "failures": [
                { "path": "C:\\a", "reason": "busy" },
                { "path": "D:\\b", "reason": "exists" }
            ]
        }));

        assert_eq!(failures.len(), 2);
        assert_eq!(failures[0].reason, "busy");
    }

    #[test]
    fn escapes_csv_values() {
        assert_eq!(csv_escape("plain"), "plain");
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("a\"b"), "\"a\"\"b\"");
    }

    #[test]
    fn sanitizes_export_file_names() {
        assert_eq!(safe_file_name("bad:name*"), "bad-name-");
    }
}
