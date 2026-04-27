import { invoke } from "@tauri-apps/api/core";
import { PathScanSummary } from "./scan";

export type CleanupPreviewItem = {
  id: string;
  name: string;
  risk: string;
  action: string;
  estimatedBytes: number;
  fileCount: number;
  skippedCount: number;
  paths: PathScanSummary[];
};

export type CleanupPreview = {
  estimatedBytes: number;
  fileCount: number;
  skippedCount: number;
  requiresConfirmation: boolean;
  executable: boolean;
  items: CleanupPreviewItem[];
};

export type CleanupPlanDraft = {
  id: string;
  status: string;
  executable: boolean;
  estimatedBytes: number;
  fileCount: number;
  skippedCount: number;
  summary: string;
};

export type QuarantineCleanupResult = {
  id: string;
  status: string;
  movedCount: number;
  skippedCount: number;
  movedBytes: number;
  quarantinePath: string;
  failures: Array<{
    path: string;
    reason: string;
  }>;
};

export function getCleanupPreview() {
  return invoke<CleanupPreview>("get_cleanup_preview");
}

export function createCleanupPlanDraft() {
  return invoke<CleanupPlanDraft>("create_cleanup_plan_draft");
}

export function executeTempQuarantineCleanup() {
  return invoke<QuarantineCleanupResult>("execute_temp_quarantine_cleanup", {
    confirmation: "MOVE_TO_QUARANTINE",
  });
}
