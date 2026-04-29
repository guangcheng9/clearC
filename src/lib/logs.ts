import { invoke } from "@tauri-apps/api/core";

export type LogEntryView = {
  id: string;
  operationType: string;
  status: string;
  createdAtMs: number;
  rollbackable: boolean;
  canRollback: boolean;
  summary: string;
  failureCount: number;
};

export type LogSummary = {
  totalOperations: number;
  rollbackableOperations: number;
  plannedOperations: number;
  failedOperations: number;
  recent: LogEntryView[];
};

export type RollbackResult = {
  id: string;
  rollbackOf: string;
  status: string;
  restoredCount: number;
  skippedCount: number;
  failures: Array<{
    path: string;
    reason: string;
  }>;
};

export function getLogSummary() {
  return invoke<LogSummary>("get_log_summary");
}

export function rollbackQuarantineCleanup(operationId: string) {
  return invoke<RollbackResult>("rollback_quarantine_cleanup", {
    operationId,
    confirmation: "RESTORE_FROM_QUARANTINE",
  });
}

export type FailureExportResult = {
  operationId: string;
  exportedCount: number;
  path: string;
};

export function exportOperationFailures(operationId: string) {
  return invoke<FailureExportResult>("export_operation_failures", {
    operationId,
  });
}

export function openFailureExportFolder() {
  return invoke<string>("open_failure_export_folder");
}
