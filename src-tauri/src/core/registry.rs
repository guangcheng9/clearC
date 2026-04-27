pub const SHELL_FOLDERS_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders";

#[derive(Debug, Clone)]
pub struct ShellFolderValue {
    pub value: String,
    pub value_type: String,
}

#[cfg(windows)]
pub fn read_user_shell_folder(value_name: &str) -> Result<Option<String>, String> {
    Ok(read_user_shell_folder_value(value_name)?.map(|value| value.value))
}

#[cfg(windows)]
pub fn read_user_shell_folder_value(value_name: &str) -> Result<Option<ShellFolderValue>, String> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey(SHELL_FOLDERS_KEY)
        .map_err(|err| format!("failed to open User Shell Folders: {}", err))?;

    match key.get_raw_value(value_name) {
        Ok(raw) => {
            let value = raw_value_to_string(&raw.bytes)?;
            Ok(Some(ShellFolderValue {
                value,
                value_type: format!("{:?}", raw.vtype),
            }))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("failed to read {}: {}", value_name, err)),
    }
}

#[cfg(not(windows))]
pub fn read_user_shell_folder(_value_name: &str) -> Result<Option<String>, String> {
    Ok(None)
}

#[cfg(not(windows))]
pub fn read_user_shell_folder_value(_value_name: &str) -> Result<Option<ShellFolderValue>, String> {
    Ok(None)
}

#[cfg(windows)]
pub fn write_user_shell_folder(value_name: &str, value: &str) -> Result<(), String> {
    use winreg::{
        enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_EXPAND_SZ, REG_SZ},
        RegKey, RegValue,
    };

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(SHELL_FOLDERS_KEY, KEY_READ | KEY_WRITE)
        .map_err(|err| format!("failed to open User Shell Folders for write: {}", err))?;

    let value_type = key
        .get_raw_value(value_name)
        .map(|raw| raw.vtype)
        .unwrap_or(REG_EXPAND_SZ);
    let value_type = if value_type == REG_SZ {
        REG_SZ
    } else {
        REG_EXPAND_SZ
    };
    let raw = RegValue {
        bytes: string_to_raw_value(value),
        vtype: value_type,
    };

    key.set_raw_value(value_name, &raw)
        .map_err(|err| format!("failed to write {}: {}", value_name, err))
}

#[cfg(not(windows))]
pub fn write_user_shell_folder(_value_name: &str, _value: &str) -> Result<(), String> {
    Err("writing User Shell Folders is only supported on Windows".to_string())
}

#[cfg(windows)]
fn raw_value_to_string(bytes: &[u8]) -> Result<String, String> {
    let mut words = Vec::new();
    for chunk in bytes.chunks_exact(2) {
        let word = u16::from_le_bytes([chunk[0], chunk[1]]);
        if word == 0 {
            break;
        }
        words.push(word);
    }

    String::from_utf16(&words).map_err(|err| format!("failed to decode registry string: {}", err))
}

#[cfg(windows)]
fn string_to_raw_value(value: &str) -> Vec<u8> {
    value
        .encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}
