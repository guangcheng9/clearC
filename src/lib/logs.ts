import { invoke } from "@tauri-apps/api/core";

export type LogEntryView = {
  id: string;
  operationType: string;
  status: string;
  createdAtMs: number;
  rollbackable: boolean;
  canRollback: boolean;
  summary: string;
};

export type LogSummary = {
  totalOperations: number;
  rollbackableOperations: number;
  plannedOperations: number;
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
