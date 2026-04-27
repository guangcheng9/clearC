use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, fs, path::PathBuf};

pub const RULES_DIR: &str = "rules";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupRule {
    pub id: String,
    pub name: String,
    pub paths: Vec<String>,
    pub risk: String,
    pub action: String,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationRule {
    pub id: String,
    pub name: String,
    pub source: String,
    pub risk: String,
    pub strategy: String,
    pub rollback: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevSpaceRule {
    pub id: String,
    pub name: String,
    pub paths: Vec<String>,
    pub risk: String,
    pub category: String,
    pub preferred_action: String,
    pub preferred_move: String,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub fallback: String,
    pub rollback: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleCatalog {
    pub rules_dir: String,
    pub cleanup: Vec<CleanupRule>,
    pub migration: Vec<MigrationRule>,
    pub devspace: Vec<DevSpaceRule>,
}

pub fn load_catalog() -> Result<RuleCatalog, String> {
    let rules_dir = find_rules_dir()?;

    Ok(RuleCatalog {
        rules_dir: rules_dir.display().to_string(),
        cleanup: read_rules_file(&rules_dir, "cleanup.rules.json")?,
        migration: read_rules_file(&rules_dir, "migration.rules.json")?,
        devspace: read_rules_file(&rules_dir, "devspace.rules.json")?,
    })
}

fn read_rules_file<T>(rules_dir: &PathBuf, file_name: &str) -> Result<Vec<T>, String>
where
    T: for<'de> Deserialize<'de>,
{
    let path = rules_dir.join(file_name);
    let content = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {}", path.display(), err))?;

    serde_json::from_str(&content)
        .map_err(|err| format!("failed to parse {}: {}", path.display(), err))
}

fn find_rules_dir() -> Result<PathBuf, String> {
    let current_dir = env::current_dir().map_err(|err| format!("failed to read cwd: {}", err))?;
    let candidates = [
        current_dir.join(RULES_DIR),
        current_dir.join("..").join(RULES_DIR),
        current_dir.join("..").join("..").join(RULES_DIR),
    ];

    candidates
        .into_iter()
        .find(|path| path.is_dir())
        .ok_or_else(|| format!("failed to locate {} directory", RULES_DIR))
}
