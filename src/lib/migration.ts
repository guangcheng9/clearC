import { invoke } from "@tauri-apps/api/core";

export type UserFolderTarget = {
  id: string;
  name: string;
  registryValue: string;
  configuredPath: string;
  resolvedPath: string;
  fallbackPath: string;
  exists: boolean;
  onSystemDrive: boolean;
  risk: string;
  strategy: string;
  rollback: boolean;
  status: string;
};

export type MigrationPrecheck = {
  folderId: string;
  name: string;
  sourcePath: string;
  targetPath: string;
  sourceExists: boolean;
  targetExists: boolean;
  targetOnSystemDrive: boolean;
  targetDriveExists: boolean;
  sourceBytes: number;
  targetFreeBytes: number;
  hasEnoughSpace: boolean;
  writableStatus: string;
  canContinue: boolean;
  blockers: string[];
  warnings: string[];
};

export type UserFolderMigrationPlan = {
  id: string;
  folderId: string;
  name: string;
  registryValue: string;
  originalRegistryValue?: string | null;
  sourcePath: string;
  targetPath: string;
  sourceBytes: number;
  fileCount: number;
  canExecute: boolean;
  blockers: string[];
  warnings: string[];
};

export type UserFolderMigrationResult = {
  id: string;
  status: string;
  folderId: string;
  name: string;
  movedCount: number;
  skippedCount: number;
  movedBytes: number;
  sourcePath: string;
  targetPath: string;
  failures: Array<{
    path: string;
    reason: string;
  }>;
};

export type UserFolderMigrationRollbackResult = {
  id: string;
  rollbackOf: string;
  status: string;
  restoredCount: number;
  skippedCount: number;
  restoredRegistry: boolean;
  failures: Array<{
    path: string;
    reason: string;
  }>;
};

export function getUserFolderTargets() {
  return invoke<UserFolderTarget[]>("get_user_folder_targets");
}

export function precheckUserFolderMigration(folderId: string, targetRoot: string) {
  return invoke<MigrationPrecheck>("precheck_user_folder_migration", {
    folderId,
    targetRoot,
  });
}

export function createUserFolderMigrationPlan(folderId: string, targetRoot: string) {
  return invoke<UserFolderMigrationPlan>("create_user_folder_migration_plan", {
    folderId,
    targetRoot,
  });
}

export function executeUserFolderMigration(folderId: string, targetRoot: string, confirmation: string) {
  return invoke<UserFolderMigrationResult>("execute_user_folder_migration", {
    folderId,
    targetRoot,
    confirmation,
  });
}

export function rollbackUserFolderMigration(operationId: string) {
  return invoke<UserFolderMigrationRollbackResult>("rollback_user_folder_migration", {
    operationId,
    confirmation: "ROLLBACK_USER_FOLDER_MIGRATION",
  });
}
