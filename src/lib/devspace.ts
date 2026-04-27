import { invoke } from "@tauri-apps/api/core";
import { PathScanSummary } from "./scan";

export type DevSpaceScanResult = {
  id: string;
  name: string;
  risk: string;
  category: string;
  preferredAction: string;
  preferredMove: string;
  rollback: boolean;
  exists: boolean;
  totalBytes: number;
  fileCount: number;
  skippedCount: number;
  paths: PathScanSummary[];
};

export function scanDevSpaceTargets() {
  return invoke<DevSpaceScanResult[]>("scan_devspace_targets");
}
