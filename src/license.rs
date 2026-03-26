use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const POLAR_ORG_ID: &str = "be6f775b-bc99-41e9-b42d-f293d5400347";
const POLAR_VALIDATE_URL: &str = "https://api.polar.sh/v1/users/license-keys/validate";

// ── Local storage ────────────────────────────────────────────────────────────

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

/// Stable machine identifier used as the Polar instance name.
pub fn instance_name() -> String {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_string());
    format!("{}-{}", user, std::env::consts::OS)
}

// ── Polar API ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ValidateRequest<'a> {
    key: &'a str,
    organization_id: &'a str,
    activation_id: &'a str,
}

#[derive(Deserialize)]
struct ValidateResponse {
    #[allow(dead_code)]
    id: Option<String>,
}

/// Validates a license key against Polar. Returns Ok(()) if valid.
pub async fn validate_with_polar(key: &str) -> Result<(), String> {
    let instance = instance_name();
    let body = ValidateRequest {
        key,
        organization_id: POLAR_ORG_ID,
        activation_id: &instance,
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(POLAR_VALIDATE_URL)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    if resp.status().is_success() {
        let _: ValidateResponse = resp
            .json()
            .await
            .map_err(|e| format!("Invalid response: {e}"))?;
        Ok(())
    } else {
        let status = resp.status().as_u16();
        Err(match status {
            404 => "License key not found.".to_string(),
            422 => "License key is invalid or already activated on another device.".to_string(),
            _ => format!("Activation failed (status {status})."),
        })
    }
}
