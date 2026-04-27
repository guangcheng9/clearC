import { invoke } from "@tauri-apps/api/core";

export type CleanupRule = {
  id: string;
  name: string;
  paths: string[];
  risk: string;
  action: string;
  exclude: string[];
};

export type MigrationRule = {
  id: string;
  name: string;
  source: string;
  risk: string;
  strategy: string;
  rollback: boolean;
};

export type DevSpaceRule = {
  id: string;
  name: string;
  paths: string[];
  risk: string;
  category: string;
  preferredAction: string;
  preferredMove: string;
  env: Record<string, string>;
  fallback: string;
  rollback: boolean;
};

export type RuleCatalog = {
  rulesDir: string;
  cleanup: CleanupRule[];
  migration: MigrationRule[];
  devspace: DevSpaceRule[];
};

export function getRuleCatalog() {
  return invoke<RuleCatalog>("get_rule_catalog");
}
