import { invoke } from "@tauri-apps/api/core";

export type DiskOverview = {
  drive: string;
  totalBytes: number;
  freeBytes: number;
  usedBytes: number;
};

export type PathScanSummary = {
  rawPath: string;
  resolvedPath: string;
  exists: boolean;
  accessible: boolean;
  fileCount: number;
  dirCount: number;
  totalBytes: number;
  skippedCount: number;
  error?: string | null;
};

export type CleanupScanResult = {
  id: string;
  name: string;
  risk: string;
  action: string;
  paths: PathScanSummary[];
  totalBytes: number;
  fileCount: number;
  skippedCount: number;
};

export function getDiskOverview() {
  return invoke<DiskOverview>("get_disk_overview");
}

export function scanCleanupRules() {
  return invoke<CleanupScanResult[]>("scan_cleanup_rules");
}

export function formatBytes(bytes: number) {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }

  const units = ["B", "KB", "MB", "GB", "TB"];
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** index;

  return `${value.toFixed(value >= 10 || index === 0 ? 0 : 1)} ${units[index]}`;
}
