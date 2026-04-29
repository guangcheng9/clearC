use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};

static CANCELLED_TASKS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

#[tauri::command]
pub fn request_task_cancel(task: String) -> Result<(), String> {
    cancelled_tasks()
        .lock()
        .map_err(|err| format!("failed to lock cancel registry: {}", err))?
        .insert(task);
    Ok(())
}

pub fn clear_task_cancel(task: &str) {
    if let Ok(mut tasks) = cancelled_tasks().lock() {
        tasks.remove(task);
    }
}

pub fn is_task_cancelled(task: &str) -> bool {
    cancelled_tasks()
        .lock()
        .map(|tasks| tasks.contains(task))
        .unwrap_or(false)
}

fn cancelled_tasks() -> &'static Mutex<HashSet<String>> {
    CANCELLED_TASKS.get_or_init(|| Mutex::new(HashSet::new()))
}
