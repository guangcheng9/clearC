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

export type DevSpaceEnvVarChange = {
  name: string;
  originalValue?: string | null;
  newValue: string;
  targetPath: string;
};

export type DevSpaceEnvMigrationPlan = {
  id: string;
  targetId: string;
  name: string;
  targetRoot: string;
  canExecute: boolean;
  blockers: string[];
  warnings: string[];
  variables: DevSpaceEnvVarChange[];
};

export type DevSpaceEnvMigrationResult = {
  id: string;
  status: string;
  targetId: string;
  name: string;
  changedCount: number;
  skippedCount: number;
  failures: Array<{
    path: string;
    reason: string;
  }>;
};

export type DevSpaceEnvRollbackResult = {
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

export type DevSpaceJunctionPlanItem = {
  rawSourcePath: string;
  sourcePath: string;
  targetPath: string;
  sourceExists: boolean;
  sourceAccessible: boolean;
  sourceBytes: number;
  targetExists: boolean;
  targetDriveExists: boolean;
  targetFreeBytes: number;
  alreadyJunction: boolean;
  blockers: string[];
  warnings: string[];
};

export type DevSpaceJunctionPlan = {
  id: string;
  targetId: string;
  name: string;
  targetRoot: string;
  canExecute: boolean;
  blockers: string[];
  warnings: string[];
  items: DevSpaceJunctionPlanItem[];
};

export type DevSpaceJunctionMigrationResult = {
  id: string;
  status: string;
  targetId: string;
  name: string;
  movedCount: number;
  skippedCount: number;
  movedBytes: number;
  failures: Array<{
    path: string;
    reason: string;
  }>;
};

export type DevSpaceJunctionRollbackResult = {
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

export function scanDevSpaceTargets() {
  return invoke<DevSpaceScanResult[]>("scan_devspace_targets");
}

export function createDevSpaceEnvMigrationPlan(targetId: string, targetRoot: string) {
  return invoke<DevSpaceEnvMigrationPlan>("create_devspace_env_migration_plan", {
    targetId,
    targetRoot,
  });
}

export function executeDevSpaceEnvMigration(targetId: string, targetRoot: string, confirmation: string) {
  return invoke<DevSpaceEnvMigrationResult>("execute_devspace_env_migration", {
    targetId,
    targetRoot,
    confirmation,
  });
}

export function rollbackDevSpaceEnvMigration(operationId: string) {
  return invoke<DevSpaceEnvRollbackResult>("rollback_devspace_env_migration", {
    operationId,
    confirmation: "ROLLBACK_DEVSPACE_ENV",
  });
}

export function createDevSpaceJunctionPlan(targetId: string, targetRoot: string) {
  return invoke<DevSpaceJunctionPlan>("create_devspace_junction_plan", {
    targetId,
    targetRoot,
  });
}

export function executeDevSpaceJunctionMigration(targetId: string, targetRoot: string, confirmation: string) {
  return invoke<DevSpaceJunctionMigrationResult>("execute_devspace_junction_migration", {
    targetId,
    targetRoot,
    confirmation,
  });
}

export function rollbackDevSpaceJunctionMigration(operationId: string) {
  return invoke<DevSpaceJunctionRollbackResult>("rollback_devspace_junction_migration", {
    operationId,
    confirmation: "ROLLBACK_DEVSPACE_JUNCTION",
  });
}
