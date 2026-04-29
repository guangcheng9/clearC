use crate::commands::tasks::{clear_task_cancel, is_task_cancelled};
use crate::core::{
    paths::{expand_path, system_drive_root},
    registry::{read_user_shell_folder, read_user_shell_folder_value, write_user_shell_folder},
    rules::{load_catalog, MigrationRule},
    scan::{disk_space, scan_rule_path},
};
use crate::storage::logs::{
    append_operation, new_operation_id, now_ms, read_operations, OperationLogEntry,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Emitter};

const MIGRATION_CONFIRMATION: &str = "MIGRATE_USER_FOLDER";
const MIGRATION_ROLLBACK_CONFIRMATION: &str = "ROLLBACK_USER_FOLDER_MIGRATION";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationTarget {
    id: String,
    name: String,
    strategy: String,
    rollback: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFolderTarget {
    id: String,
    name: String,
    registry_value: String,
    configured_path: String,
    resolved_path: String,
    fallback_path: String,
    exists: bool,
    on_system_drive: bool,
    risk: String,
    strategy: String,
    rollback: bool,
    status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationPrecheck {
    folder_id: String,
    name: String,
    source_path: String,
    target_path: String,
    source_exists: bool,
    target_exists: bool,
    target_on_system_drive: bool,
    target_drive_exists: bool,
    source_bytes: u64,
    target_free_bytes: u64,
    has_enough_space: bool,
    writable_status: String,
    can_continue: bool,
    blockers: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFolderMigrationPlan {
    id: String,
    folder_id: String,
    name: String,
    registry_value: String,
    original_registry_value: Option<String>,
    original_registry_type: Option<String>,
    source_path: String,
    target_path: String,
    source_bytes: u64,
    file_count: u64,
    can_execute: bool,
    blockers: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFolderMigrationResult {
    id: String,
    status: String,
    folder_id: String,
    name: String,
    moved_count: u64,
    skipped_count: u64,
    moved_bytes: u64,
    source_path: String,
    target_path: String,
    failures: Vec<MigrationFailure>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFolderMigrationRollbackResult {
    id: String,
    rollback_of: String,
    status: String,
    restored_count: u64,
    skipped_count: u64,
    restored_registry: bool,
    failures: Vec<MigrationFailure>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationFailure {
    path: String,
    reason: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MovedMigrationItem {
    original_path: String,
    target_path: String,
    bytes: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MovedMigrationItemForRollback {
    original_path: String,
    target_path: String,
}

struct ControlledMoveResult {
    bytes: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationTaskProgressEvent {
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
    current_file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_file_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_file_total_bytes: Option<u64>,
}

#[tauri::command]
pub fn get_migration_targets() -> Result<Vec<MigrationTarget>, String> {
    let catalog = load_catalog()?;

    Ok(catalog
        .migration
        .into_iter()
        .map(|rule| MigrationTarget {
            id: rule.id,
            name: rule.name,
            strategy: rule.strategy,
            rollback: rule.rollback,
        })
        .collect())
}

#[tauri::command]
pub fn get_user_folder_targets() -> Result<Vec<UserFolderTarget>, String> {
    let catalog = load_catalog()?;

    catalog
        .migration
        .into_iter()
        .map(build_user_folder_target)
        .collect()
}

#[tauri::command]
pub async fn precheck_user_folder_migration(
    app: AppHandle,
    folder_id: String,
    target_root: String,
) -> Result<MigrationPrecheck, String> {
    tauri::async_runtime::spawn_blocking(move || {
        emit_migration_task_progress(
            &app,
            "migration-precheck",
            0,
            1,
            "开始迁移预检查".to_string(),
            "running",
        );
        let result = precheck_user_folder_migration_blocking(folder_id, target_root);
        emit_migration_task_progress(
            &app,
            "migration-precheck",
            1,
            1,
            "迁移预检查完成".to_string(),
            "completed",
        );
        result
    })
    .await
    .map_err(|err| format!("migration precheck task failed: {}", err))?
}

#[tauri::command]
pub async fn create_user_folder_migration_plan(
    app: AppHandle,
    folder_id: String,
    target_root: String,
) -> Result<UserFolderMigrationPlan, String> {
    tauri::async_runtime::spawn_blocking(move || {
        emit_migration_task_progress(
            &app,
            "migration-plan",
            0,
            1,
            "开始生成迁移计划".to_string(),
            "running",
        );
        let result = create_user_folder_migration_plan_blocking(folder_id, target_root);
        emit_migration_task_progress(
            &app,
            "migration-plan",
            1,
            1,
            "迁移计划生成完成".to_string(),
            "completed",
        );
        result
    })
    .await
    .map_err(|err| format!("migration plan task failed: {}", err))?
}

#[tauri::command]
pub async fn execute_user_folder_migration(
    app: AppHandle,
    folder_id: String,
    target_root: String,
    confirmation: String,
) -> Result<UserFolderMigrationResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        execute_user_folder_migration_blocking(app, folder_id, target_root, confirmation)
    })
    .await
    .map_err(|err| format!("migration task failed: {}", err))?
}

#[tauri::command]
pub async fn rollback_user_folder_migration(
    app: AppHandle,
    operation_id: String,
    confirmation: String,
) -> Result<UserFolderMigrationRollbackResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        rollback_user_folder_migration_blocking(app, operation_id, confirmation)
    })
    .await
    .map_err(|err| format!("migration rollback task failed: {}", err))?
}

fn precheck_user_folder_migration_blocking(
    folder_id: String,
    target_root: String,
) -> Result<MigrationPrecheck, String> {
    let catalog = load_catalog()?;
    let rule = catalog
        .migration
        .into_iter()
        .find(|rule| rule.id == folder_id)
        .ok_or_else(|| format!("migration rule {} not found", folder_id))?;
    let target = build_user_folder_target(rule.clone())?;
    let source = PathBuf::from(&target.resolved_path);
    let target_path = PathBuf::from(target_root).join(safe_folder_dir_name(&rule.id, &rule.name));
    let target_drive = drive_root_for_path(&target_path).unwrap_or_else(system_drive_root);
    let target_drive_exists = PathBuf::from(&target_drive).exists();
    let (target_total, target_free_bytes) = disk_space(&target_drive).unwrap_or((0, 0));
    let source_summary = scan_rule_path(&target.configured_path);
    let source_bytes = source_summary.total_bytes;
    let target_exists = target_path.exists();
    let target_on_system_drive = is_on_system_drive(&target_path);
    let has_enough_space = target_free_bytes > source_bytes;
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    if !source.exists() {
        blockers.push("源目录不存在".to_string());
    }

    if target_exists {
        blockers.push("目标目录已存在".to_string());
    }

    if !target_drive_exists || target_total == 0 {
        blockers.push("目标盘不存在或无法读取容量".to_string());
    }

    if target_on_system_drive {
        warnings.push("目标路径仍在系统盘".to_string());
    }

    if !has_enough_space {
        blockers.push("目标盘剩余空间不足".to_string());
    }

    let can_continue = blockers.is_empty();

    Ok(MigrationPrecheck {
        folder_id: rule.id,
        name: rule.name,
        source_path: source.display().to_string(),
        target_path: target_path.display().to_string(),
        source_exists: source.exists(),
        target_exists,
        target_on_system_drive,
        target_drive_exists,
        source_bytes,
        target_free_bytes,
        has_enough_space,
        writable_status: "not-verified".to_string(),
        can_continue,
        blockers,
        warnings,
    })
}

fn create_user_folder_migration_plan_blocking(
    folder_id: String,
    target_root: String,
) -> Result<UserFolderMigrationPlan, String> {
    let precheck = precheck_user_folder_migration_blocking(folder_id.clone(), target_root)?;
    let target = get_user_folder_targets()?
        .into_iter()
        .find(|target| target.id == folder_id)
        .ok_or_else(|| format!("user folder {} not found", folder_id))?;
    let operation_id = new_operation_id("user-folder-migration-plan");
    let original_registry = read_user_shell_folder_value(&target.registry_value)?;

    let plan = UserFolderMigrationPlan {
        id: operation_id.clone(),
        folder_id: folder_id.clone(),
        name: precheck.name,
        registry_value: target.registry_value.clone(),
        original_registry_value: original_registry
            .as_ref()
            .map(|registry_value| registry_value.value.clone()),
        original_registry_type: original_registry.map(|registry_value| registry_value.value_type),
        source_path: precheck.source_path,
        target_path: precheck.target_path,
        source_bytes: precheck.source_bytes,
        file_count: scan_rule_path(&target.configured_path).file_count,
        can_execute: precheck.can_continue,
        blockers: precheck.blockers,
        warnings: precheck.warnings,
    };

    append_operation(&OperationLogEntry {
        id: operation_id,
        operation_type: "user-folder-migration-plan".to_string(),
        status: "planned".to_string(),
        created_at_ms: now_ms(),
        rollbackable: false,
        summary: format!(
            "用户目录迁移计划：{} -> {}",
            plan.source_path, plan.target_path
        ),
        details: json!({
            "folderId": plan.folder_id,
            "name": plan.name,
            "registryValue": plan.registry_value,
            "originalRegistryValue": plan.original_registry_value,
            "originalRegistryType": plan.original_registry_type,
            "sourcePath": plan.source_path,
            "targetPath": plan.target_path,
            "sourceBytes": plan.source_bytes,
            "fileCount": plan.file_count,
            "canExecute": plan.can_execute,
            "blockers": plan.blockers,
            "warnings": plan.warnings,
        }),
    })?;

    Ok(plan)
}

fn execute_user_folder_migration_blocking(
    app: AppHandle,
    folder_id: String,
    target_root: String,
    confirmation: String,
) -> Result<UserFolderMigrationResult, String> {
    clear_task_cancel("migration-execute");
    if confirmation != MIGRATION_CONFIRMATION {
        return Err("invalid migration confirmation".to_string());
    }

    let plan = create_user_folder_migration_plan_blocking(folder_id.clone(), target_root)?;
    if !plan.can_execute {
        return Err(format!(
            "migration precheck failed: {}",
            plan.blockers.join("，")
        ));
    }

    let source_dir = PathBuf::from(&plan.source_path);
    let target_dir = PathBuf::from(&plan.target_path);
    fs::create_dir_all(&target_dir).map_err(|err| {
        format!(
            "failed to create target dir {}: {}",
            target_dir.display(),
            err
        )
    })?;

    let mut moved_count = 0u64;
    let mut skipped_count = 0u64;
    let mut moved_bytes = 0u64;
    let mut moved_items = Vec::new();
    let mut failures = Vec::new();

    let entries = fs::read_dir(&source_dir).map_err(|err| {
        format!(
            "failed to read source dir {}: {}",
            source_dir.display(),
            err
        )
    })?;
    let entries: Vec<_> = entries.collect();
    let total = entries.len() as u64;
    emit_migration_task_progress(
        &app,
        "migration-execute",
        0,
        total,
        format!("开始迁移 {}", plan.name),
        "running",
    );

    for (index, entry) in entries.into_iter().enumerate() {
        if is_task_cancelled("migration-execute") {
            emit_migration_task_progress(
                &app,
                "migration-execute",
                index as u64,
                total,
                "用户目录迁移已取消".to_string(),
                "cancelled",
            );
            clear_task_cancel("migration-execute");
            return Err("migration-execute cancelled".to_string());
        }

        emit_migration_task_progress_with_stats(
            &app,
            "migration-execute",
            index as u64,
            total,
            format!("正在迁移 {}", plan.name),
            "running",
            MigrationProgressStats {
                processed_items: Some(moved_count.saturating_add(skipped_count)),
                moved_count: Some(moved_count),
                skipped_count: Some(skipped_count),
                processed_bytes: Some(moved_bytes),
                failure_count: Some(failures.len() as u64),
                ..MigrationProgressStats::default()
            },
        );
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                skipped_count += 1;
                failures.push(MigrationFailure {
                    path: source_dir.display().to_string(),
                    reason: err.to_string(),
                });
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
        let destination = target_dir.join(file_name);

        if destination.exists() {
            skipped_count += 1;
            failures.push(MigrationFailure {
                path: destination.display().to_string(),
                reason: "target item already exists".to_string(),
            });
            continue;
        }

        let size = path_size(&source).unwrap_or(0);
        let copy_progress = CopyProgressContext {
            app: &app,
            task: "migration-execute",
            item_index: index as u64,
            total_items: total,
            processed_items: moved_count.saturating_add(skipped_count),
            moved_count,
            skipped_count,
            processed_bytes: moved_bytes,
            failure_count: failures.len() as u64,
            verb: "正在复制",
        };
        match move_path_controlled_with_cancel(
            &source,
            &destination,
            Some("migration-execute"),
            Some(&copy_progress),
        ) {
            Ok(move_result) => {
                moved_count += 1;
                moved_bytes = moved_bytes.saturating_add(move_result.bytes);
                moved_items.push(MovedMigrationItem {
                    original_path: source.display().to_string(),
                    target_path: destination.display().to_string(),
                    bytes: move_result.bytes.max(size),
                });
            }
            Err(err) => {
                if err.contains("cancelled") {
                    emit_migration_task_progress(
                        &app,
                        "migration-execute",
                        index as u64,
                        total,
                        "用户目录迁移已取消".to_string(),
                        "cancelled",
                    );
                    clear_task_cancel("migration-execute");
                    return Err(err);
                }
                skipped_count += 1;
                failures.push(MigrationFailure {
                    path: source.display().to_string(),
                    reason: err.to_string(),
                });
            }
        }

        emit_migration_task_progress_with_stats(
            &app,
            "migration-execute",
            index as u64 + 1,
            total,
            format!("已处理 {}", source.display()),
            "running",
            MigrationProgressStats {
                processed_items: Some(moved_count.saturating_add(skipped_count)),
                moved_count: Some(moved_count),
                skipped_count: Some(skipped_count),
                processed_bytes: Some(moved_bytes),
                failure_count: Some(failures.len() as u64),
                ..MigrationProgressStats::default()
            },
        );
    }

    let mut registry_updated = false;
    if let Err(err) = write_user_shell_folder(&plan.registry_value, &plan.target_path) {
        failures.push(MigrationFailure {
            path: plan.registry_value.clone(),
            reason: format!("failed to update User Shell Folders: {}", err),
        });
    } else {
        registry_updated = true;
    }

    let status = if !registry_updated {
        "completed-with-registry-failure"
    } else if failures.is_empty() {
        "completed"
    } else {
        "completed-with-skips"
    }
    .to_string();
    let operation_id = new_operation_id("user-folder-migration");
    let summary = format!(
        "用户目录迁移：{} -> {}，移动 {} 项，跳过 {} 项",
        plan.source_path, plan.target_path, moved_count, skipped_count
    );

    append_operation(&OperationLogEntry {
        id: operation_id.clone(),
        operation_type: "user-folder-migration".to_string(),
        status: status.clone(),
        created_at_ms: now_ms(),
        rollbackable: true,
        summary,
        details: json!({
            "folderId": plan.folder_id,
            "name": plan.name,
            "registryValue": plan.registry_value,
            "originalRegistryValue": plan.original_registry_value,
            "originalRegistryType": plan.original_registry_type,
            "newRegistryValue": plan.target_path,
            "registryUpdated": registry_updated,
            "sourcePath": plan.source_path,
            "targetPath": plan.target_path,
            "movedCount": moved_count,
            "skippedCount": skipped_count,
            "movedBytes": moved_bytes,
            "movedItems": moved_items,
            "failures": failures,
        }),
    })?;

    emit_migration_task_progress_with_stats(
        &app,
        "migration-execute",
        total,
        total,
        "用户目录迁移完成".to_string(),
        "completed",
        MigrationProgressStats {
            processed_items: Some(moved_count.saturating_add(skipped_count)),
            moved_count: Some(moved_count),
            skipped_count: Some(skipped_count),
            processed_bytes: Some(moved_bytes),
            failure_count: Some(failures.len() as u64),
            ..MigrationProgressStats::default()
        },
    );
    clear_task_cancel("migration-execute");

    Ok(UserFolderMigrationResult {
        id: operation_id,
        status,
        folder_id: plan.folder_id,
        name: plan.name,
        moved_count,
        skipped_count,
        moved_bytes,
        source_path: plan.source_path,
        target_path: plan.target_path,
        failures,
    })
}

fn rollback_user_folder_migration_blocking(
    app: AppHandle,
    operation_id: String,
    confirmation: String,
) -> Result<UserFolderMigrationRollbackResult, String> {
    clear_task_cancel("migration-rollback");
    if confirmation != MIGRATION_ROLLBACK_CONFIRMATION {
        return Err("invalid migration rollback confirmation".to_string());
    }

    let entries = read_operations()?;
    let rollback_targets = migration_rollback_targets(&entries);
    if rollback_targets.contains(&operation_id) {
        return Err("migration already has a rollback record".to_string());
    }

    let entry = entries
        .iter()
        .find(|entry| entry.id == operation_id)
        .ok_or_else(|| format!("migration {} not found", operation_id))?;

    if entry.operation_type != "user-folder-migration" || !entry.rollbackable {
        return Err("operation is not a rollbackable user folder migration".to_string());
    }

    let registry_value = entry
        .details
        .get("registryValue")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "migration log missing registryValue".to_string())?
        .to_string();
    let source_path = entry
        .details
        .get("sourcePath")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "migration log missing sourcePath".to_string())?
        .to_string();
    let original_registry_value = entry
        .details
        .get("originalRegistryValue")
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
        .unwrap_or(source_path);
    let moved_items: Vec<MovedMigrationItemForRollback> = serde_json::from_value(
        entry
            .details
            .get("movedItems")
            .cloned()
            .ok_or_else(|| "migration log missing movedItems".to_string())?,
    )
    .map_err(|err| format!("failed to parse movedItems: {}", err))?;

    let mut restored_count = 0u64;
    let mut skipped_count = 0u64;
    let mut failures = Vec::new();
    let total = moved_items.len() as u64;

    emit_migration_task_progress(
        &app,
        "migration-rollback",
        0,
        total,
        "开始回滚用户目录迁移".to_string(),
        "running",
    );

    for (index, item) in moved_items.into_iter().enumerate() {
        if is_task_cancelled("migration-rollback") {
            emit_migration_task_progress(
                &app,
                "migration-rollback",
                index as u64,
                total,
                "用户目录迁移回滚已取消".to_string(),
                "cancelled",
            );
            clear_task_cancel("migration-rollback");
            return Err("migration-rollback cancelled".to_string());
        }

        emit_migration_task_progress_with_stats(
            &app,
            "migration-rollback",
            index as u64,
            total,
            "正在回滚用户目录迁移".to_string(),
            "running",
            MigrationProgressStats {
                processed_items: Some(restored_count.saturating_add(skipped_count)),
                moved_count: Some(restored_count),
                skipped_count: Some(skipped_count),
                processed_bytes: None,
                failure_count: Some(failures.len() as u64),
                ..MigrationProgressStats::default()
            },
        );
        let source = PathBuf::from(&item.target_path);
        let destination = PathBuf::from(&item.original_path);

        if !source.exists() {
            skipped_count += 1;
            failures.push(MigrationFailure {
                path: source.display().to_string(),
                reason: "migrated item does not exist".to_string(),
            });
            continue;
        }

        if destination.exists() {
            skipped_count += 1;
            failures.push(MigrationFailure {
                path: destination.display().to_string(),
                reason: "original path already exists".to_string(),
            });
            continue;
        }

        if let Some(parent) = destination.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                skipped_count += 1;
                failures.push(MigrationFailure {
                    path: parent.display().to_string(),
                    reason: err.to_string(),
                });
                continue;
            }
        }

        let copy_progress = CopyProgressContext {
            app: &app,
            task: "migration-rollback",
            item_index: index as u64,
            total_items: total,
            processed_items: restored_count.saturating_add(skipped_count),
            moved_count: restored_count,
            skipped_count,
            processed_bytes: 0,
            failure_count: failures.len() as u64,
            verb: "正在恢复",
        };
        match move_path_controlled_with_cancel(
            &source,
            &destination,
            Some("migration-rollback"),
            Some(&copy_progress),
        ) {
            Ok(_) => restored_count += 1,
            Err(err) => {
                if err.contains("cancelled") {
                    emit_migration_task_progress(
                        &app,
                        "migration-rollback",
                        index as u64,
                        total,
                        "用户目录迁移回滚已取消".to_string(),
                        "cancelled",
                    );
                    clear_task_cancel("migration-rollback");
                    return Err(err);
                }
                skipped_count += 1;
                failures.push(MigrationFailure {
                    path: source.display().to_string(),
                    reason: err.to_string(),
                });
            }
        }

        emit_migration_task_progress_with_stats(
            &app,
            "migration-rollback",
            index as u64 + 1,
            total,
            format!("已处理 {}", source.display()),
            "running",
            MigrationProgressStats {
                processed_items: Some(restored_count.saturating_add(skipped_count)),
                moved_count: Some(restored_count),
                skipped_count: Some(skipped_count),
                processed_bytes: None,
                failure_count: Some(failures.len() as u64),
                ..MigrationProgressStats::default()
            },
        );
    }

    let mut restored_registry = false;
    if let Err(err) = write_user_shell_folder(&registry_value, &original_registry_value) {
        failures.push(MigrationFailure {
            path: registry_value.clone(),
            reason: format!("failed to restore User Shell Folders: {}", err),
        });
    } else {
        restored_registry = true;
    }
    let rollback_id = new_operation_id("user-folder-migration-rollback");
    let status = if !restored_registry {
        "rolled-back-with-registry-failure"
    } else if failures.is_empty() {
        "rolled-back"
    } else {
        "rolled-back-with-skips"
    }
    .to_string();

    append_operation(&OperationLogEntry {
        id: rollback_id.clone(),
        operation_type: "user-folder-migration-rollback".to_string(),
        status: status.clone(),
        created_at_ms: now_ms(),
        rollbackable: false,
        summary: format!(
            "用户目录迁移回滚：恢复 {} 项，跳过 {} 项",
            restored_count, skipped_count
        ),
        details: json!({
            "rollbackOf": operation_id,
            "restoredCount": restored_count,
            "skippedCount": skipped_count,
            "restoredRegistry": restored_registry,
            "failures": failures,
        }),
    })?;

    emit_migration_task_progress_with_stats(
        &app,
        "migration-rollback",
        total,
        total,
        "用户目录迁移回滚完成".to_string(),
        "completed",
        MigrationProgressStats {
            processed_items: Some(restored_count.saturating_add(skipped_count)),
            moved_count: Some(restored_count),
            skipped_count: Some(skipped_count),
            processed_bytes: None,
            failure_count: Some(failures.len() as u64),
            ..MigrationProgressStats::default()
        },
    );
    clear_task_cancel("migration-rollback");

    Ok(UserFolderMigrationRollbackResult {
        id: rollback_id,
        rollback_of: operation_id,
        status,
        restored_count,
        skipped_count,
        restored_registry,
        failures,
    })
}

fn emit_migration_task_progress(
    app: &AppHandle,
    task: &str,
    current: u64,
    total: u64,
    label: String,
    status: &str,
) {
    emit_migration_task_progress_with_stats(
        app,
        task,
        current,
        total,
        label,
        status,
        MigrationProgressStats::default(),
    );
}

#[derive(Default)]
struct MigrationProgressStats {
    processed_items: Option<u64>,
    moved_count: Option<u64>,
    skipped_count: Option<u64>,
    processed_bytes: Option<u64>,
    failure_count: Option<u64>,
    current_file_path: Option<String>,
    current_file_bytes: Option<u64>,
    current_file_total_bytes: Option<u64>,
}

fn emit_migration_task_progress_with_stats(
    app: &AppHandle,
    task: &str,
    current: u64,
    total: u64,
    label: String,
    status: &str,
    stats: MigrationProgressStats,
) {
    let _ = app.emit(
        "clearc://task-progress",
        MigrationTaskProgressEvent {
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
            current_file_path: stats.current_file_path,
            current_file_bytes: stats.current_file_bytes,
            current_file_total_bytes: stats.current_file_total_bytes,
        },
    );
}

fn build_user_folder_target(rule: MigrationRule) -> Result<UserFolderTarget, String> {
    let registry_value = registry_value_name(&rule.id).to_string();
    let configured_path =
        read_user_shell_folder(&registry_value)?.unwrap_or_else(|| rule.source.clone());
    let resolved = expand_path(&configured_path);
    let fallback = expand_path(&rule.source);
    let exists = resolved.exists();
    let on_system_drive = is_on_system_drive(&resolved);
    let status = if exists { "detected" } else { "missing" }.to_string();

    Ok(UserFolderTarget {
        id: rule.id,
        name: rule.name,
        registry_value,
        configured_path,
        resolved_path: resolved.display().to_string(),
        fallback_path: fallback.display().to_string(),
        exists,
        on_system_drive,
        risk: rule.risk,
        strategy: rule.strategy,
        rollback: rule.rollback,
        status,
    })
}

fn migration_rollback_targets(entries: &[OperationLogEntry]) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| entry.operation_type == "user-folder-migration-rollback")
        .filter_map(|entry| {
            entry
                .details
                .get("rollbackOf")
                .and_then(|value| value.as_str())
                .map(ToString::to_string)
        })
        .collect()
}

fn registry_value_name(id: &str) -> &str {
    match id {
        "desktop" => "Desktop",
        "downloads" => "{374DE290-123F-4565-9164-39C4925E467B}",
        "documents" => "Personal",
        "pictures" => "My Pictures",
        "videos" => "My Video",
        "music" => "My Music",
        _ => id,
    }
}

fn is_on_system_drive(path: &PathBuf) -> bool {
    let system_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
    let path_text = path.display().to_string();
    path_text
        .to_ascii_lowercase()
        .starts_with(&system_drive.to_ascii_lowercase())
}

fn safe_folder_dir_name(id: &str, name: &str) -> String {
    match id {
        "desktop" => "Desktop".to_string(),
        "downloads" => "Downloads".to_string(),
        "documents" => "Documents".to_string(),
        "pictures" => "Pictures".to_string(),
        "videos" => "Videos".to_string(),
        "music" => "Music".to_string(),
        _ => name.to_string(),
    }
}

fn drive_root_for_path(path: &Path) -> Option<String> {
    let text = path.display().to_string();
    let bytes = text.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' {
        Some(format!("{}\\", &text[0..2]))
    } else {
        None
    }
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

#[cfg(test)]
fn move_path_controlled(source: &Path, destination: &Path) -> Result<ControlledMoveResult, String> {
    move_path_controlled_with_cancel(source, destination, None, None)
}

fn move_path_controlled_with_cancel(
    source: &Path,
    destination: &Path,
    cancel_task: Option<&str>,
    progress: Option<&CopyProgressContext>,
) -> Result<ControlledMoveResult, String> {
    if destination.exists() {
        return Err("target item already exists".to_string());
    }

    let bytes = path_size(&source.to_path_buf()).unwrap_or(0);
    match fs::rename(source, destination) {
        Ok(()) => return Ok(ControlledMoveResult { bytes }),
        Err(rename_err) => {
            let metadata = fs::symlink_metadata(source)
                .map_err(|err| format!("failed to read {}: {}", source.display(), err))?;
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "symlink migration is not supported after rename failed: {}",
                    rename_err
                ));
            }
        }
    }

    if let Err(err) = copy_path_verified(source, destination, cancel_task, progress) {
        let _ = remove_path(destination);
        return Err(err);
    }
    let copied_bytes = path_size(&destination.to_path_buf()).unwrap_or(0);
    if copied_bytes != bytes {
        let _ = remove_path(destination);
        return Err(format!(
            "copied size mismatch: expected {} bytes, copied {} bytes",
            bytes, copied_bytes
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

    Ok(ControlledMoveResult { bytes })
}

fn copy_path_verified(
    source: &Path,
    destination: &Path,
    cancel_task: Option<&str>,
    progress: Option<&CopyProgressContext>,
) -> Result<(), String> {
    check_copy_cancel(cancel_task)?;
    let metadata = fs::symlink_metadata(source)
        .map_err(|err| format!("failed to read {}: {}", source.display(), err))?;

    if metadata.is_file() {
        return copy_file_verified(source, destination, cancel_task, progress);
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
                "target item already exists: {}",
                destination_child.display()
            ));
        }
        copy_path_verified(&source_child, &destination_child, cancel_task, progress)?;
    }

    Ok(())
}

struct CopyProgressContext<'a> {
    app: &'a AppHandle,
    task: &'a str,
    item_index: u64,
    total_items: u64,
    processed_items: u64,
    moved_count: u64,
    skipped_count: u64,
    processed_bytes: u64,
    failure_count: u64,
    verb: &'a str,
}

fn copy_file_verified(
    source: &Path,
    destination: &Path,
    cancel_task: Option<&str>,
    progress: Option<&CopyProgressContext>,
) -> Result<(), String> {
    check_copy_cancel(cancel_task)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {}", parent.display(), err))?;
    }

    let mut reader = fs::File::open(source)
        .map_err(|err| format!("failed to open {}: {}", source.display(), err))?;
    let mut writer = fs::File::create(destination)
        .map_err(|err| format!("failed to create {}: {}", destination.display(), err))?;
    let mut buffer = vec![0u8; 1024 * 1024];
    let total_bytes = fs::symlink_metadata(source)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let mut copied_bytes = 0u64;

    loop {
        check_copy_cancel(cancel_task)?;
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
        if let Some(progress) = progress {
            emit_copy_progress(progress, source, copied_bytes, total_bytes);
        }
    }

    writer
        .flush()
        .map_err(|err| format!("failed to flush {}: {}", destination.display(), err))?;
    Ok(())
}

fn emit_copy_progress(
    progress: &CopyProgressContext,
    source: &Path,
    copied_bytes: u64,
    total_bytes: u64,
) {
    emit_migration_task_progress_with_stats(
        progress.app,
        progress.task,
        progress.item_index,
        progress.total_items,
        format!("{} {}", progress.verb, source.display()),
        "running",
        MigrationProgressStats {
            processed_items: Some(progress.processed_items),
            moved_count: Some(progress.moved_count),
            skipped_count: Some(progress.skipped_count),
            processed_bytes: Some(progress.processed_bytes.saturating_add(copied_bytes)),
            failure_count: Some(progress.failure_count),
            current_file_path: Some(source.display().to_string()),
            current_file_bytes: Some(copied_bytes),
            current_file_total_bytes: Some(total_bytes),
            ..MigrationProgressStats::default()
        },
    );
}

fn check_copy_cancel(cancel_task: Option<&str>) -> Result<(), String> {
    if let Some(task) = cancel_task {
        if is_task_cancelled(task) {
            return Err(format!("{} cancelled", task));
        }
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

#[cfg(test)]
mod tests {
    use super::{
        copy_file_verified, drive_root_for_path, move_path_controlled, registry_value_name,
        safe_folder_dir_name,
    };
    use crate::commands::tasks::{clear_task_cancel, request_task_cancel};
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn maps_shell_folder_registry_values() {
        assert_eq!(
            registry_value_name("downloads"),
            "{374DE290-123F-4565-9164-39C4925E467B}"
        );
        assert_eq!(registry_value_name("documents"), "Personal");
        assert_eq!(registry_value_name("pictures"), "My Pictures");
    }

    #[test]
    fn maps_target_folder_names() {
        assert_eq!(safe_folder_dir_name("downloads", "下载"), "Downloads");
        assert_eq!(safe_folder_dir_name("desktop", "桌面"), "Desktop");
    }

    #[test]
    fn detects_windows_drive_root() {
        assert_eq!(
            drive_root_for_path(&PathBuf::from("D:\\Data\\Downloads")),
            Some("D:\\".to_string())
        );
    }

    #[test]
    fn controlled_move_skips_existing_target() {
        let root = test_dir("clearc-existing-target");
        let source = root.join("source.txt");
        let destination = root.join("destination.txt");
        fs::create_dir_all(&root).unwrap();
        fs::write(&source, "source").unwrap();
        fs::write(&destination, "target").unwrap();

        let result = move_path_controlled(&source, &destination);

        assert!(result.is_err());
        assert!(source.exists());
        assert_eq!(fs::read_to_string(destination).unwrap(), "target");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn controlled_move_moves_directory_contents() {
        let root = test_dir("clearc-move-dir");
        let source = root.join("source-dir");
        let destination = root.join("destination-dir");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::write(source.join("nested").join("file.txt"), "hello").unwrap();

        move_path_controlled(&source, &destination).unwrap();

        assert!(!source.exists());
        assert_eq!(
            fs::read_to_string(destination.join("nested").join("file.txt")).unwrap(),
            "hello"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cancellable_copy_checks_cancel_before_copying() {
        let root = test_dir("clearc-copy-cancel");
        let source = root.join("source.txt");
        let destination = root.join("destination.txt");
        fs::create_dir_all(&root).unwrap();
        fs::write(&source, "source").unwrap();

        request_task_cancel("test-copy-cancel".to_string()).unwrap();
        let result = copy_file_verified(&source, &destination, Some("test-copy-cancel"), None);

        assert!(result.is_err());
        assert!(!destination.exists());
        clear_task_cancel("test-copy-cancel");
        let _ = fs::remove_dir_all(root);
    }

    fn test_dir(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{}-{}", name, suffix))
    }
}
