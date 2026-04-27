use std::{env, path::PathBuf};

pub const USER_PROFILE_VAR: &str = "%USERPROFILE%";

pub fn expand_path(input: &str) -> PathBuf {
    let mut value = input.to_string();

    for (key, env_value) in env::vars() {
        let token = format!("%{}%", key);
        value = value.replace(&token, &env_value);
    }

    if value == "$Recycle.Bin" {
        let system_drive = env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
        value = format!("{}\\$Recycle.Bin", system_drive);
    }

    PathBuf::from(value)
}

pub fn system_drive_root() -> String {
    let drive = env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
    format!("{}\\", drive)
}
