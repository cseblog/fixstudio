use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// License data stored on disk after successful activation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StoredLicense {
    pub key: String,
    pub instance_id: String,
}

pub fn license_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home).join("Library/Application Support/AiFIXParser/license.json")
    }
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        PathBuf::from(appdata).join("AiFIXParser/license.json")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home).join(".config/aifixparser/license.json")
    }
}

pub fn load_license() -> Option<StoredLicense> {
    let data = std::fs::read_to_string(license_path()).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_license(lic: &StoredLicense) {
    if let Some(dir) = license_path().parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string(lic) {
        let _ = std::fs::write(license_path(), json);
    }
}

pub fn clear_license() {
    let _ = std::fs::remove_file(license_path());
}

/// A stable machine identifier used as the Lemon Squeezy instance name.
pub fn instance_name() -> String {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_string());
    format!("{}-{}", user, std::env::consts::OS)
}
