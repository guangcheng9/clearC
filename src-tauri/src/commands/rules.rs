use crate::core::rules::{load_catalog, RuleCatalog};

#[tauri::command]
pub fn get_rule_catalog() -> Result<RuleCatalog, String> {
    load_catalog()
}
