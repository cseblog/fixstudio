use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Whop credentials ──────────────────────────────────────────────────────────
// API key: Whop → Developer Dashboard → Company API Keys → Create
// Permission needed: member:basic:read
const WHOP_API_KEY:      &str = "apik_0V9cFRk4ZURA7_C4706365_C_69aa27a9cfefb9222006072921092c186d3f295a7e16500af1cc17a1aa24d8";
const WHOP_COMPANY_ID:   &str = "biz_3S9JyWeJxWSJS0";
const WHOP_PRODUCT_ID:   &str = "prod_ZEXdRPkg2eG7X";
const WHOP_MEMBERSHIPS:  &str = "https://api.whop.com/api/v2/memberships";

// ── Local storage ─────────────────────────────────────────────────────────────

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

/// Stable machine identifier stored alongside the license key.
pub fn instance_name() -> String {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_string());
    format!("{}-{}", user, std::env::consts::OS)
}

// ── Whop API ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct MembershipsResponse {
    data: Vec<Membership>,
}

#[derive(Deserialize)]
struct Membership {
    license_key: Option<String>,
}

/// Validates a license key against Whop. Returns `Ok(())` if an active
/// membership with that key exists for this product.
pub async fn validate_license(key: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(WHOP_MEMBERSHIPS)
        .bearer_auth(WHOP_API_KEY)
        .query(&[
            ("company_id",      WHOP_COMPANY_ID),
            ("product_ids[]",   WHOP_PRODUCT_ID),
            ("statuses[]",      "active"),
            ("license_key",     key),
        ])
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    let status = resp.status().as_u16();
    if status == 401 {
        return Err("Activation failed: invalid API configuration.".to_string());
    }

    let payload: MembershipsResponse = resp
        .json()
        .await
        .map_err(|e| format!("Invalid response: {e}"))?;

    let found = payload.data.iter()
        .any(|m| m.license_key.as_deref() == Some(key));

    if found {
        Ok(())
    } else {
        Err("License key not found or subscription is not active.".to_string())
    }
}
