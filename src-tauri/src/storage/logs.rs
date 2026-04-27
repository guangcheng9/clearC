use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub const OPERATION_LOG_FILE: &str = "operations.jsonl";
pub const DATA_DIR: &str = ".clearc";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationLogEntry {
    pub id: String,
    pub operation_type: String,
    pub status: String,
    pub created_at_ms: u128,
    pub rollbackable: bool,
    pub summary: String,
    pub details: Value,
}

pub fn append_operation(entry: &OperationLogEntry) -> Result<(), String> {
    let path = log_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create log dir {}: {}", parent.display(), err))?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|err| format!("failed to open log file {}: {}", path.display(), err))?;

    let line = serde_json::to_string(entry)
        .map_err(|err| format!("failed to serialize operation log: {}", err))?;
    writeln!(file, "{}", line)
        .map_err(|err| format!("failed to write log file {}: {}", path.display(), err))
}

pub fn read_operations() -> Result<Vec<OperationLogEntry>, String> {
    let path = log_file_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(&path)
        .map_err(|err| format!("failed to open log file {}: {}", path.display(), err))?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for line in reader.lines() {
        let line =
            line.map_err(|err| format!("failed to read log file {}: {}", path.display(), err))?;
        if line.trim().is_empty() {
            continue;
        }

        let entry = serde_json::from_str(&line)
            .map_err(|err| format!("failed to parse log entry: {}", err))?;
        entries.push(entry);
    }

    Ok(entries)
}

pub fn new_operation_id(prefix: &str) -> String {
    format!("{}-{}", prefix, now_ms())
}

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn log_file_path() -> Result<PathBuf, String> {
    Ok(data_dir_path()?.join(OPERATION_LOG_FILE))
}

pub fn data_dir_path() -> Result<PathBuf, String> {
    let current_dir =
        std::env::current_dir().map_err(|err| format!("failed to read cwd: {}", err))?;
    Ok(current_dir.join(DATA_DIR))
}
