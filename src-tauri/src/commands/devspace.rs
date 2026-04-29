use crate::commands::tasks::{clear_task_cancel, is_task_cancelled};
use crate::core::{
    paths::system_drive_root,
    rules::load_catalog,
    scan::{disk_space, scan_rule_path, PathScanSummary},
};
use crate::storage::logs::{
    append_operation, new_operation_id, now_ms, read_operations, OperationLogEntry,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{AppHandle, Emitter};

const DEVSPACE_ENV_CONFIRMATION: &str = "MIGRATE_DEVSPACE_ENV";
const DEVSPACE_ENV_ROLLBACK_CONFIRMATION: &str = "ROLLBACK_DEVSPACE_ENV";
const DEVSPACE_JUNCTION_CONFIRMATION: &str = "CREATE_DEVSPACE_JUNCTION";
const DEVSPACE_JUNCTION_ROLLBACK_CONFIRMATION: &str = "ROLLBACK_DEVSPACE_JUNCTION";

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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevSpaceTaskProgressEvent {
    task: String,
    current: u64,
    total: u64,
    label: String,
    status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevSpaceEnvMigrationPlan {
    id: String,
    target_id: String,
    name: String,
    target_root: String,
    can_execute: bool,
    blockers: Vec<String>,
    warnings: Vec<String>,
    variables: Vec<DevSpaceEnvVarChange>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DevSpaceEnvVarChange {
    name: String,
    original_value: Option<String>,
    new_value: String,
    target_path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevSpaceEnvMigrationResult {
    id: String,
    status: String,
    target_id: String,
    name: String,
    changed_count: u64,
    skipped_count: u64,
    failures: Vec<DevSpaceEnvFailure>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevSpaceEnvRollbackResult {
    id: String,
    rollback_of: String,
    status: String,
    restored_count: u64,
    skipped_count: u64,
    failures: Vec<DevSpaceEnvFailure>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevSpaceEnvFailure {
    path: String,
    reason: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevSpaceJunctionPlan {
    id: String,
    target_id: String,
    name: String,
    target_root: String,
    can_execute: bool,
    blockers: Vec<String>,
    warnings: Vec<String>,
    items: Vec<DevSpaceJunctionPlanItem>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevSpaceJunctionPlanItem {
    raw_source_path: String,
    source_path: String,
    target_path: String,
    source_exists: bool,
    source_accessible: bool,
    source_bytes: u64,
    target_exists: bool,
    target_drive_exists: bool,
    target_free_bytes: u64,
    already_junction: bool,
    blockers: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevSpaceJunctionMigrationResult {
    id: String,
    status: String,
    target_id: String,
    name: String,
    moved_count: u64,
    skipped_count: u64,
    moved_bytes: u64,
    failures: Vec<DevSpaceJunctionFailure>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevSpaceJunctionRollbackResult {
    id: String,
    rollback_of: String,
    status: String,
    restored_count: u64,
    skipped_count: u64,
    failures: Vec<DevSpaceJunctionFailure>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MovedJunctionItem {
    source_path: String,
    target_path: String,
    bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevSpaceJunctionFailure {
    path: String,
    reason: String,
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
pub async fn scan_devspace_targets(app: AppHandle) -> Result<Vec<DevSpaceScanResult>, String> {
    tauri::async_runtime::spawn_blocking(move || scan_devspace_targets_blocking(app))
        .await
        .map_err(|err| format!("devspace scan task failed: {}", err))?
}

#[tauri::command]
pub async fn create_devspace_env_migration_plan(
    target_id: String,
    target_root: String,
) -> Result<DevSpaceEnvMigrationPlan, String> {
    tauri::async_runtime::spawn_blocking(move || {
        create_devspace_env_migration_plan_blocking(target_id, target_root)
    })
    .await
    .map_err(|err| format!("devspace env migration plan task failed: {}", err))?
}

#[tauri::command]
pub async fn execute_devspace_env_migration(
    target_id: String,
    target_root: String,
    confirmation: String,
) -> Result<DevSpaceEnvMigrationResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        execute_devspace_env_migration_blocking(target_id, target_root, confirmation)
    })
    .await
    .map_err(|err| format!("devspace env migration task failed: {}", err))?
}

#[tauri::command]
pub async fn rollback_devspace_env_migration(
    operation_id: String,
    confirmation: String,
) -> Result<DevSpaceEnvRollbackResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        rollback_devspace_env_migration_blocking(operation_id, confirmation)
    })
    .await
    .map_err(|err| format!("devspace env rollback task failed: {}", err))?
}

#[tauri::command]
pub async fn create_devspace_junction_plan(
    target_id: String,
    target_root: String,
) -> Result<DevSpaceJunctionPlan, String> {
    tauri::async_runtime::spawn_blocking(move || {
        create_devspace_junction_plan_blocking(target_id, target_root)
    })
    .await
    .map_err(|err| format!("devspace junction plan task failed: {}", err))?
}

#[tauri::command]
pub async fn execute_devspace_junction_migration(
    target_id: String,
    target_root: String,
    confirmation: String,
) -> Result<DevSpaceJunctionMigrationResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        execute_devspace_junction_migration_blocking(target_id, target_root, confirmation)
    })
    .await
    .map_err(|err| format!("devspace junction migration task failed: {}", err))?
}

#[tauri::command]
pub async fn rollback_devspace_junction_migration(
    operation_id: String,
    confirmation: String,
) -> Result<DevSpaceJunctionRollbackResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        rollback_devspace_junction_migration_blocking(operation_id, confirmation)
    })
    .await
    .map_err(|err| format!("devspace junction rollback task failed: {}", err))?
}

fn scan_devspace_targets_blocking(app: AppHandle) -> Result<Vec<DevSpaceScanResult>, String> {
    clear_task_cancel("scan-devspace-targets");
    let catalog = load_catalog()?;
    let total = catalog.devspace.len() as u64;
    let mut results = Vec::new();

    emit_devspace_task_progress(&app, 0, total, "准备扫描开发工具".to_string(), "running");

    for (index, rule) in catalog.devspace.into_iter().enumerate() {
        if is_task_cancelled("scan-devspace-targets") {
            emit_devspace_task_progress(
                &app,
                index as u64,
                total,
                "开发工具扫描已取消".to_string(),
                "cancelled",
            );
            clear_task_cancel("scan-devspace-targets");
            return Err("scan-devspace-targets cancelled".to_string());
        }

        emit_devspace_task_progress(
            &app,
            index as u64,
            total,
            format!("正在扫描 {}", rule.name),
            "running",
        );

        let paths: Vec<PathScanSummary> =
            rule.paths.iter().map(|path| scan_rule_path(path)).collect();
        let exists = paths.iter().any(|path| path.exists);
        let total_bytes = paths.iter().map(|path| path.total_bytes).sum();
        let file_count = paths.iter().map(|path| path.file_count).sum();
        let skipped_count = paths.iter().map(|path| path.skipped_count).sum();

        results.push(DevSpaceScanResult {
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
        });

        emit_devspace_task_progress(
            &app,
            index as u64 + 1,
            total,
            "开发工具扫描项完成".to_string(),
            "running",
        );
    }

    emit_devspace_task_progress(
        &app,
        total,
        total,
        "开发工具扫描完成".to_string(),
        "completed",
    );
    clear_task_cancel("scan-devspace-targets");

    Ok(results)
}

fn emit_devspace_task_progress(
    app: &AppHandle,
    current: u64,
    total: u64,
    label: String,
    status: &str,
) {
    let _ = app.emit(
        "clearc://task-progress",
        DevSpaceTaskProgressEvent {
            task: "scan-devspace-targets".to_string(),
            current,
            total,
            label,
            status: status.to_string(),
        },
    );
}

fn create_devspace_env_migration_plan_blocking(
    target_id: String,
    target_root: String,
) -> Result<DevSpaceEnvMigrationPlan, String> {
    let catalog = load_catalog()?;
    let rule = catalog
        .devspace
        .into_iter()
        .find(|rule| rule.id == target_id)
        .ok_or_else(|| format!("devspace rule {} not found", target_id))?;

    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    if rule.risk == "high" {
        blockers.push("高风险目录不允许自动迁移".to_string());
    }

    if rule.env.is_empty() || rule.preferred_move != "env" {
        blockers.push("该工具不支持环境变量迁移".to_string());
    }

    let target_root_path = PathBuf::from(&target_root);
    if is_on_system_drive(&target_root_path) {
        warnings.push("目标根目录仍在系统盘".to_string());
    }

    let mut variables = Vec::new();
    for name in sorted_env_names(&rule.env) {
        let target_path = target_root_path.join(devspace_env_dir_name(&name));
        variables.push(DevSpaceEnvVarChange {
            name: name.clone(),
            original_value: read_user_env_var(&name)?,
            new_value: target_path.display().to_string(),
            target_path: target_path.display().to_string(),
        });
    }

    let operation_id = new_operation_id("devspace-env-migration-plan");
    let plan = DevSpaceEnvMigrationPlan {
        id: operation_id.clone(),
        target_id: rule.id,
        name: rule.name,
        target_root,
        can_execute: blockers.is_empty(),
        blockers,
        warnings,
        variables,
    };

    append_operation(&OperationLogEntry {
        id: operation_id,
        operation_type: "devspace-env-migration-plan".to_string(),
        status: "planned".to_string(),
        created_at_ms: now_ms(),
        rollbackable: false,
        summary: format!("开发工具环境变量迁移计划：{}", plan.name),
        details: json!({
            "targetId": plan.target_id,
            "name": plan.name,
            "targetRoot": plan.target_root,
            "canExecute": plan.can_execute,
            "blockers": plan.blockers,
            "warnings": plan.warnings,
            "variables": plan.variables,
        }),
    })?;

    Ok(plan)
}

fn execute_devspace_env_migration_blocking(
    target_id: String,
    target_root: String,
    confirmation: String,
) -> Result<DevSpaceEnvMigrationResult, String> {
    if confirmation != DEVSPACE_ENV_CONFIRMATION {
        return Err("invalid devspace env migration confirmation".to_string());
    }

    let plan = create_devspace_env_migration_plan_blocking(target_id, target_root)?;
    if !plan.can_execute {
        return Err(format!(
            "devspace env migration precheck failed: {}",
            plan.blockers.join("，")
        ));
    }

    let mut changed_count = 0u64;
    let mut skipped_count = 0u64;
    let mut failures = Vec::new();

    for variable in &plan.variables {
        if let Err(err) = fs::create_dir_all(&variable.target_path) {
            skipped_count += 1;
            failures.push(DevSpaceEnvFailure {
                path: variable.target_path.clone(),
                reason: format!("failed to create target dir: {}", err),
            });
            continue;
        }

        match write_user_env_var(&variable.name, &variable.new_value) {
            Ok(()) => changed_count += 1,
            Err(err) => {
                skipped_count += 1;
                failures.push(DevSpaceEnvFailure {
                    path: variable.name.clone(),
                    reason: err,
                });
            }
        }
    }

    let status = if failures.is_empty() {
        "completed"
    } else {
        "completed-with-skips"
    }
    .to_string();
    let operation_id = new_operation_id("devspace-env-migration");

    append_operation(&OperationLogEntry {
        id: operation_id.clone(),
        operation_type: "devspace-env-migration".to_string(),
        status: status.clone(),
        created_at_ms: now_ms(),
        rollbackable: true,
        summary: format!(
            "开发工具环境变量迁移：{}，修改 {} 项，跳过 {} 项",
            plan.name, changed_count, skipped_count
        ),
        details: json!({
            "targetId": plan.target_id,
            "name": plan.name,
            "targetRoot": plan.target_root,
            "changedCount": changed_count,
            "skippedCount": skipped_count,
            "variables": plan.variables,
            "failures": failures,
        }),
    })?;

    Ok(DevSpaceEnvMigrationResult {
        id: operation_id,
        status,
        target_id: plan.target_id,
        name: plan.name,
        changed_count,
        skipped_count,
        failures,
    })
}

fn rollback_devspace_env_migration_blocking(
    operation_id: String,
    confirmation: String,
) -> Result<DevSpaceEnvRollbackResult, String> {
    if confirmation != DEVSPACE_ENV_ROLLBACK_CONFIRMATION {
        return Err("invalid devspace env rollback confirmation".to_string());
    }

    let entries = read_operations()?;
    let rollback_targets = devspace_env_rollback_targets(&entries);
    if rollback_targets.contains(&operation_id) {
        return Err("devspace env migration already has a rollback record".to_string());
    }

    let entry = entries
        .iter()
        .find(|entry| entry.id == operation_id)
        .ok_or_else(|| format!("devspace env migration {} not found", operation_id))?;

    if entry.operation_type != "devspace-env-migration" || !entry.rollbackable {
        return Err("operation is not a rollbackable devspace env migration".to_string());
    }

    let variables: Vec<DevSpaceEnvVarChange> = serde_json::from_value(
        entry
            .details
            .get("variables")
            .cloned()
            .ok_or_else(|| "devspace env log missing variables".to_string())?,
    )
    .map_err(|err| format!("failed to parse variables: {}", err))?;

    let mut restored_count = 0u64;
    let mut skipped_count = 0u64;
    let mut failures = Vec::new();
    for variable in variables {
        let result = match variable.original_value {
            Some(original_value) => write_user_env_var(&variable.name, &original_value),
            None => delete_user_env_var(&variable.name),
        };

        match result {
            Ok(()) => restored_count += 1,
            Err(err) => {
                skipped_count += 1;
                failures.push(DevSpaceEnvFailure {
                    path: variable.name,
                    reason: err,
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
    let rollback_id = new_operation_id("devspace-env-migration-rollback");

    append_operation(&OperationLogEntry {
        id: rollback_id.clone(),
        operation_type: "devspace-env-migration-rollback".to_string(),
        status: status.clone(),
        created_at_ms: now_ms(),
        rollbackable: false,
        summary: format!(
            "开发工具环境变量迁移回滚：恢复 {} 项，跳过 {} 项",
            restored_count, skipped_count
        ),
        details: json!({
            "rollbackOf": operation_id,
            "restoredCount": restored_count,
            "skippedCount": skipped_count,
            "failures": failures,
        }),
    })?;

    Ok(DevSpaceEnvRollbackResult {
        id: rollback_id,
        rollback_of: operation_id,
        status,
        restored_count,
        skipped_count,
        failures,
    })
}

fn create_devspace_junction_plan_blocking(
    target_id: String,
    target_root: String,
) -> Result<DevSpaceJunctionPlan, String> {
    let catalog = load_catalog()?;
    let rule = catalog
        .devspace
        .into_iter()
        .find(|rule| rule.id == target_id)
        .ok_or_else(|| format!("devspace rule {} not found", target_id))?;

    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    if rule.risk == "high" {
        blockers.push("高风险目录不允许自动生成 Junction 执行预案".to_string());
    }

    if rule.preferred_move != "junction" && rule.fallback != "junction" {
        blockers.push("该工具不支持 Junction 迁移预案".to_string());
    }

    if rule.preferred_action == "readonly" {
        blockers.push("该规则当前推荐只读，Junction 预案仅用于评估，不开放执行".to_string());
    }

    if rule.preferred_action == "cleanup" {
        blockers.push("该规则当前推荐清理，不开放 Junction 执行".to_string());
    }

    let target_root_path = PathBuf::from(&target_root);
    if is_on_system_drive(&target_root_path) {
        warnings.push("目标根目录仍在系统盘".to_string());
    }

    let items: Vec<DevSpaceJunctionPlanItem> = rule
        .paths
        .iter()
        .map(|raw_path| build_junction_plan_item(&target_root_path, &rule.id, raw_path))
        .collect();

    if items.is_empty() {
        blockers.push("规则没有可检测路径".to_string());
    }

    if items.iter().any(|item| !item.blockers.is_empty()) {
        blockers.push("存在路径级阻塞项".to_string());
    }

    let operation_id = new_operation_id("devspace-junction-plan");
    let plan = DevSpaceJunctionPlan {
        id: operation_id.clone(),
        target_id: rule.id,
        name: rule.name,
        target_root,
        can_execute: blockers.is_empty(),
        blockers,
        warnings,
        items,
    };

    append_operation(&OperationLogEntry {
        id: operation_id,
        operation_type: "devspace-junction-plan".to_string(),
        status: "planned".to_string(),
        created_at_ms: now_ms(),
        rollbackable: false,
        summary: format!("开发工具 Junction 迁移预案：{}", plan.name),
        details: json!({
            "targetId": plan.target_id,
            "name": plan.name,
            "targetRoot": plan.target_root,
            "canExecute": plan.can_execute,
            "blockers": plan.blockers,
            "warnings": plan.warnings,
            "items": plan.items,
        }),
    })?;

    Ok(plan)
}

fn execute_devspace_junction_migration_blocking(
    target_id: String,
    target_root: String,
    confirmation: String,
) -> Result<DevSpaceJunctionMigrationResult, String> {
    if confirmation != DEVSPACE_JUNCTION_CONFIRMATION {
        return Err("invalid devspace junction confirmation".to_string());
    }

    let plan = create_devspace_junction_plan_blocking(target_id, target_root)?;
    if !plan.can_execute {
        return Err(format!(
            "devspace junction precheck failed: {}",
            plan.blockers.join("，")
        ));
    }

    let mut moved_count = 0u64;
    let mut skipped_count = 0u64;
    let mut moved_bytes = 0u64;
    let mut moved_items = Vec::new();
    let mut failures = Vec::new();

    for item in &plan.items {
        if !item.blockers.is_empty() {
            skipped_count += 1;
            failures.push(DevSpaceJunctionFailure {
                path: item.source_path.clone(),
                reason: item.blockers.join("，"),
            });
            continue;
        }

        let source = PathBuf::from(&item.source_path);
        let target = PathBuf::from(&item.target_path);

        match execute_single_junction_item(&source, &target, item.source_bytes) {
            Ok(bytes) => {
                moved_count += 1;
                moved_bytes = moved_bytes.saturating_add(bytes);
                moved_items.push(MovedJunctionItem {
                    source_path: item.source_path.clone(),
                    target_path: item.target_path.clone(),
                    bytes,
                });
            }
            Err(err) => {
                skipped_count += 1;
                failures.push(DevSpaceJunctionFailure {
                    path: item.source_path.clone(),
                    reason: err,
                });
            }
        }
    }

    let status = if failures.is_empty() {
        "completed"
    } else {
        "completed-with-skips"
    }
    .to_string();
    let operation_id = new_operation_id("devspace-junction-migration");

    append_operation(&OperationLogEntry {
        id: operation_id.clone(),
        operation_type: "devspace-junction-migration".to_string(),
        status: status.clone(),
        created_at_ms: now_ms(),
        rollbackable: moved_count > 0,
        summary: format!(
            "开发工具 Junction 迁移：{}，迁移 {} 项，跳过 {} 项",
            plan.name, moved_count, skipped_count
        ),
        details: json!({
            "targetId": plan.target_id,
            "name": plan.name,
            "targetRoot": plan.target_root,
            "movedCount": moved_count,
            "skippedCount": skipped_count,
            "movedBytes": moved_bytes,
            "movedItems": moved_items,
            "failures": failures,
        }),
    })?;

    Ok(DevSpaceJunctionMigrationResult {
        id: operation_id,
        status,
        target_id: plan.target_id,
        name: plan.name,
        moved_count,
        skipped_count,
        moved_bytes,
        failures,
    })
}

fn rollback_devspace_junction_migration_blocking(
    operation_id: String,
    confirmation: String,
) -> Result<DevSpaceJunctionRollbackResult, String> {
    if confirmation != DEVSPACE_JUNCTION_ROLLBACK_CONFIRMATION {
        return Err("invalid devspace junction rollback confirmation".to_string());
    }

    let entries = read_operations()?;
    let rollback_targets = devspace_junction_rollback_targets(&entries);
    if rollback_targets.contains(&operation_id) {
        return Err("devspace junction migration already has a rollback record".to_string());
    }

    let entry = entries
        .iter()
        .find(|entry| entry.id == operation_id)
        .ok_or_else(|| format!("devspace junction migration {} not found", operation_id))?;

    if entry.operation_type != "devspace-junction-migration" || !entry.rollbackable {
        return Err("operation is not a rollbackable devspace junction migration".to_string());
    }

    let moved_items: Vec<MovedJunctionItem> = serde_json::from_value(
        entry
            .details
            .get("movedItems")
            .cloned()
            .ok_or_else(|| "devspace junction log missing movedItems".to_string())?,
    )
    .map_err(|err| format!("failed to parse movedItems: {}", err))?;

    let mut restored_count = 0u64;
    let mut skipped_count = 0u64;
    let mut failures = Vec::new();

    for item in moved_items {
        match rollback_single_junction_item(
            &PathBuf::from(&item.source_path),
            &PathBuf::from(&item.target_path),
        ) {
            Ok(()) => restored_count += 1,
            Err(err) => {
                skipped_count += 1;
                failures.push(DevSpaceJunctionFailure {
                    path: item.source_path,
                    reason: err,
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
    let rollback_id = new_operation_id("devspace-junction-migration-rollback");

    append_operation(&OperationLogEntry {
        id: rollback_id.clone(),
        operation_type: "devspace-junction-migration-rollback".to_string(),
        status: status.clone(),
        created_at_ms: now_ms(),
        rollbackable: false,
        summary: format!(
            "开发工具 Junction 迁移回滚：恢复 {} 项，跳过 {} 项",
            restored_count, skipped_count
        ),
        details: json!({
            "rollbackOf": operation_id,
            "restoredCount": restored_count,
            "skippedCount": skipped_count,
            "failures": failures,
        }),
    })?;

    Ok(DevSpaceJunctionRollbackResult {
        id: rollback_id,
        rollback_of: operation_id,
        status,
        restored_count,
        skipped_count,
        failures,
    })
}

fn build_junction_plan_item(
    target_root: &Path,
    rule_id: &str,
    raw_path: &str,
) -> DevSpaceJunctionPlanItem {
    let source_summary = scan_rule_path(raw_path);
    let source_path = PathBuf::from(&source_summary.resolved_path);
    let target_path = target_root
        .join("junctions")
        .join(safe_path_segment(rule_id))
        .join(source_leaf_name(&source_path));
    let target_drive = drive_root_for_path(&target_path).unwrap_or_else(system_drive_root);
    let target_drive_exists = PathBuf::from(&target_drive).exists();
    let (_, target_free_bytes) = disk_space(&target_drive).unwrap_or((0, 0));
    let target_exists = target_path.exists();
    let already_junction = is_junction_or_symlink(&source_path);
    let mut blockers = Vec::new();
    let warnings = Vec::new();

    if !source_summary.exists {
        blockers.push("源路径不存在".to_string());
    }

    if !source_summary.accessible {
        blockers.push("源路径不可访问".to_string());
    }

    if target_exists {
        blockers.push("目标路径已存在".to_string());
    }

    if !target_drive_exists {
        blockers.push("目标盘不存在".to_string());
    }

    if target_free_bytes <= source_summary.total_bytes {
        blockers.push("目标盘剩余空间不足".to_string());
    }

    if already_junction {
        blockers.push("源路径已经是 Junction 或符号链接".to_string());
    }

    DevSpaceJunctionPlanItem {
        raw_source_path: raw_path.to_string(),
        source_path: source_summary.resolved_path,
        target_path: target_path.display().to_string(),
        source_exists: source_summary.exists,
        source_accessible: source_summary.accessible,
        source_bytes: source_summary.total_bytes,
        target_exists,
        target_drive_exists,
        target_free_bytes,
        already_junction,
        blockers,
        warnings,
    }
}

fn execute_single_junction_item(
    source: &Path,
    target: &Path,
    expected_bytes: u64,
) -> Result<u64, String> {
    if !source.exists() {
        return Err("source path does not exist".to_string());
    }
    if !source.is_dir() {
        return Err("source path is not a directory".to_string());
    }
    if is_junction_or_symlink(source) {
        return Err("source path is already a junction or symlink".to_string());
    }
    if target.exists() {
        return Err("target path already exists".to_string());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create target parent {}: {}",
                parent.display(),
                err
            )
        })?;
    }

    move_path_controlled(source, target, expected_bytes)?;
    if let Err(err) = create_directory_junction(source, target) {
        let restore_result = move_path_controlled(target, source, expected_bytes);
        return Err(match restore_result {
            Ok(()) => format!("failed to create junction and restored source: {}", err),
            Err(restore_err) => format!(
                "failed to create junction: {}; failed to restore source: {}",
                err, restore_err
            ),
        });
    }

    Ok(expected_bytes)
}

fn rollback_single_junction_item(source: &Path, target: &Path) -> Result<(), String> {
    if source.exists() {
        if !is_junction_or_symlink(source) {
            return Err("source path exists but is not a junction or symlink".to_string());
        }
        fs::remove_dir(source)
            .map_err(|err| format!("failed to remove junction {}: {}", source.display(), err))?;
    }

    if !target.exists() {
        return Err("target path does not exist".to_string());
    }
    if source.exists() {
        return Err("source path still exists after removing junction".to_string());
    }
    if let Some(parent) = source.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create source parent {}: {}",
                parent.display(),
                err
            )
        })?;
    }

    move_path_controlled(target, source, path_size(target).unwrap_or(0))
}

fn move_path_controlled(source: &Path, target: &Path, expected_bytes: u64) -> Result<(), String> {
    if target.exists() {
        return Err("target path already exists".to_string());
    }

    match fs::rename(source, target) {
        Ok(()) => return Ok(()),
        Err(rename_err) => {
            let metadata = fs::symlink_metadata(source)
                .map_err(|err| format!("failed to read {}: {}", source.display(), err))?;
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "symlink move is not supported after rename failed: {}",
                    rename_err
                ));
            }
        }
    }

    copy_path_verified(source, target)?;
    let copied_bytes = path_size(target).unwrap_or(0);
    if copied_bytes != expected_bytes {
        let _ = remove_path(target);
        return Err(format!(
            "copied size mismatch: expected {} bytes, copied {} bytes",
            expected_bytes, copied_bytes
        ));
    }

    if let Err(err) = remove_path(source) {
        let cleanup_result = remove_path(target);
        return Err(match cleanup_result {
            Ok(()) => format!("failed to remove source after copy: {}", err),
            Err(cleanup_err) => format!(
                "failed to remove source after copy: {}; copied target cleanup also failed: {}",
                err, cleanup_err
            ),
        });
    }

    Ok(())
}

fn copy_path_verified(source: &Path, target: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|err| format!("failed to read {}: {}", source.display(), err))?;

    if metadata.is_file() {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("failed to create {}: {}", parent.display(), err))?;
        }
        fs::copy(source, target).map_err(|err| {
            format!(
                "failed to copy {} to {}: {}",
                source.display(),
                target.display(),
                err
            )
        })?;
        return Ok(());
    }

    if !metadata.is_dir() {
        return Err("unsupported file type".to_string());
    }

    fs::create_dir_all(target)
        .map_err(|err| format!("failed to create {}: {}", target.display(), err))?;
    for entry in fs::read_dir(source)
        .map_err(|err| format!("failed to read dir {}: {}", source.display(), err))?
    {
        let entry = entry.map_err(|err| err.to_string())?;
        let source_child = entry.path();
        let target_child = target.join(entry.file_name());
        if target_child.exists() {
            return Err(format!(
                "target item already exists: {}",
                target_child.display()
            ));
        }
        copy_path_verified(&source_child, &target_child)?;
    }

    Ok(())
}

fn path_size(path: &Path) -> Result<u64, String> {
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

#[cfg(windows)]
fn create_directory_junction(source: &Path, target: &Path) -> Result<(), String> {
    let output = Command::new("cmd")
        .args([
            "/C",
            "mklink",
            "/J",
            &source.display().to_string(),
            &target.display().to_string(),
        ])
        .output()
        .map_err(|err| format!("failed to launch mklink: {}", err))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!("mklink /J failed: {}{}", stdout, stderr))
    }
}

#[cfg(not(windows))]
fn create_directory_junction(_source: &Path, _target: &Path) -> Result<(), String> {
    Err("creating directory junctions is only supported on Windows".to_string())
}

fn devspace_junction_rollback_targets(entries: &[OperationLogEntry]) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| entry.operation_type == "devspace-junction-migration-rollback")
        .filter_map(|entry| {
            entry
                .details
                .get("rollbackOf")
                .and_then(|value| value.as_str())
                .map(ToString::to_string)
        })
        .collect()
}

fn sorted_env_names(env: &HashMap<String, String>) -> Vec<String> {
    let mut names: Vec<String> = env.keys().cloned().collect();
    names.sort();
    names
}

fn devspace_env_dir_name(name: &str) -> String {
    match name {
        "ANDROID_USER_HOME" => "android".to_string(),
        "CARGO_HOME" => "cargo".to_string(),
        "RUSTUP_HOME" => "rustup".to_string(),
        _ => name.to_ascii_lowercase(),
    }
}

fn safe_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ => ch,
        })
        .collect()
}

fn source_leaf_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "source".to_string())
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

#[cfg(windows)]
fn is_junction_or_symlink(path: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn is_junction_or_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
}

fn devspace_env_rollback_targets(entries: &[OperationLogEntry]) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| entry.operation_type == "devspace-env-migration-rollback")
        .filter_map(|entry| {
            entry
                .details
                .get("rollbackOf")
                .and_then(|value| value.as_str())
                .map(ToString::to_string)
        })
        .collect()
}

fn is_on_system_drive(path: &PathBuf) -> bool {
    let system_drive = system_drive_root().to_ascii_lowercase();
    path.display()
        .to_string()
        .to_ascii_lowercase()
        .starts_with(&system_drive)
}

#[cfg(windows)]
fn read_user_env_var(name: &str) -> Result<Option<String>, String> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey("Environment")
        .map_err(|err| format!("failed to open user Environment: {}", err))?;

    match key.get_value::<String, _>(name) {
        Ok(value) => Ok(Some(value)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("failed to read {}: {}", name, err)),
    }
}

#[cfg(not(windows))]
fn read_user_env_var(name: &str) -> Result<Option<String>, String> {
    Ok(std::env::var(name).ok())
}

#[cfg(windows)]
fn write_user_env_var(name: &str, value: &str) -> Result<(), String> {
    use winreg::{
        enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
        RegKey,
    };

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .map_err(|err| format!("failed to open user Environment for write: {}", err))?;

    key.set_value(name, &value)
        .map_err(|err| format!("failed to write {}: {}", name, err))
}

#[cfg(not(windows))]
fn write_user_env_var(_name: &str, _value: &str) -> Result<(), String> {
    Err("writing user environment variables is only supported on Windows".to_string())
}

#[cfg(windows)]
fn delete_user_env_var(name: &str) -> Result<(), String> {
    use winreg::{
        enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
        RegKey,
    };

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .map_err(|err| format!("failed to open user Environment for write: {}", err))?;

    match key.delete_value(name) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("failed to delete {}: {}", name, err)),
    }
}

#[cfg(not(windows))]
fn delete_user_env_var(_name: &str) -> Result<(), String> {
    Err("deleting user environment variables is only supported on Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::{devspace_env_dir_name, safe_path_segment, sorted_env_names, source_leaf_name};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn maps_env_names_to_target_dirs() {
        assert_eq!(devspace_env_dir_name("ANDROID_USER_HOME"), "android");
        assert_eq!(devspace_env_dir_name("CARGO_HOME"), "cargo");
        assert_eq!(devspace_env_dir_name("RUSTUP_HOME"), "rustup");
    }

    #[test]
    fn sorts_env_names_for_stable_plans() {
        let mut env = HashMap::new();
        env.insert("RUSTUP_HOME".to_string(), "D:\\DevData\\rustup".to_string());
        env.insert("CARGO_HOME".to_string(), "D:\\DevData\\cargo".to_string());

        assert_eq!(
            sorted_env_names(&env),
            vec!["CARGO_HOME".to_string(), "RUSTUP_HOME".to_string()]
        );
    }

    #[test]
    fn sanitizes_junction_target_segments() {
        assert_eq!(safe_path_segment("bad:name*"), "bad-name-");
    }

    #[test]
    fn reads_source_leaf_name() {
        assert_eq!(
            source_leaf_name(&PathBuf::from("C:\\Users\\me\\.bun")),
            ".bun"
        );
    }
}
