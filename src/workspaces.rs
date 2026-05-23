//! Saved workspaces — name + file path + filter state + view mode.
//!
//! Operator opens a file daily with the same filter combo ("BAD_LP only,
//! last 30 minutes, Latency view"). Workspaces let them save that combo
//! and recall it in one click instead of retyping.
//!
//! On-disk shape (`~/.aifixparser/workspaces.json`) is intentionally
//! simple JSON so users can hand-edit / share / version-control.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

const MAX_WORKSPACES: usize = 32;

#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Workspace {
    pub name:         String,
    pub file_path:    String,
    pub view_mode:    u8,          // 0=Now 1=Timeline 2=Latency 3=Session 4=Validator
    #[serde(default)] pub f_sender:     String,
    #[serde(default)] pub f_target:     String,
    #[serde(default)] pub f_msg:        String,
    #[serde(default)] pub f_clord:      String,
    #[serde(default)] pub f_detail:     String,
    #[serde(default)] pub f_time:       String,
    #[serde(default)] pub f_time_op:    String,
    /// Optional LP-scorecard drill selection.
    #[serde(default)] pub selected_lp:  String,
    /// Optional latency-chain filter.
    #[serde(default)] pub chain_filter: String,
    #[serde(default)] pub auto_watch:   bool,
    #[serde(default)] pub follow_tail:  bool,
}

fn store_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let mut p = PathBuf::from(home);
    p.push(".aifixparser");
    let _ = std::fs::create_dir_all(&p);
    p.push("workspaces.json");
    Some(p)
}

pub fn load_from(path: &Path) -> Vec<Workspace> {
    let Ok(bytes) = std::fs::read(path) else { return Vec::new() };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

pub fn load() -> Vec<Workspace> {
    let Some(p) = store_path() else { return Vec::new() };
    load_from(&p)
}

/// Save (or replace) a workspace by `name`. Returns the persisted list.
pub fn save_to(path: &Path, ws: Workspace) -> Vec<Workspace> {
    if ws.name.trim().is_empty() {
        // Silently ignore — UI should validate but tests should also be safe.
        return load_from(path);
    }
    let mut list = load_from(path);
    list.retain(|w| w.name != ws.name);
    list.insert(0, ws);
    list.truncate(MAX_WORKSPACES);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, serde_json::to_vec_pretty(&list).unwrap_or_default());
    list
}

pub fn save(ws: Workspace) -> Vec<Workspace> {
    let Some(p) = store_path() else { return Vec::new() };
    save_to(&p, ws)
}

pub fn delete_to(path: &Path, name: &str) -> Vec<Workspace> {
    let mut list = load_from(path);
    list.retain(|w| w.name != name);
    let _ = std::fs::write(path, serde_json::to_vec_pretty(&list).unwrap_or_default());
    list
}

pub fn delete(name: &str) -> Vec<Workspace> {
    let Some(p) = store_path() else { return Vec::new() };
    delete_to(&p, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn unique_tmp() -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let mut p = std::env::temp_dir();
        p.push(format!("aifixparser_ws_test_{}_{}.json",
            std::process::id(), n));
        let _ = std::fs::remove_file(&p);
        p
    }

    fn sample() -> Workspace {
        Workspace {
            name: "EUR-BAD".into(),
            file_path: "/x/log.fix".into(),
            view_mode: 2,
            f_sender: "ME".into(),
            f_msg:    "D".into(),
            selected_lp: "BAD_LP".into(),
            auto_watch: true,
            ..Default::default()
        }
    }

    #[test]
    fn load_missing_returns_empty() {
        assert!(load_from(&unique_tmp()).is_empty());
    }

    #[test]
    fn save_persists_and_loads_round_trip() {
        let p = unique_tmp();
        let list = save_to(&p, sample());
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "EUR-BAD");
        assert_eq!(list[0].selected_lp, "BAD_LP");
        let reloaded = load_from(&p);
        assert_eq!(reloaded, list);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn save_replaces_same_name() {
        let p = unique_tmp();
        save_to(&p, sample());
        let mut updated = sample();
        updated.f_sender = "OTHER".into();
        let list = save_to(&p, updated);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].f_sender, "OTHER");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn empty_name_silently_ignored() {
        let p = unique_tmp();
        let mut bad = sample();
        bad.name = "   ".into();
        let list = save_to(&p, bad);
        assert!(list.is_empty());
    }

    #[test]
    fn delete_removes_named_entry() {
        let p = unique_tmp();
        save_to(&p, sample());
        let mut another = sample();
        another.name = "OTHER".into();
        save_to(&p, another);
        let after = delete_to(&p, "EUR-BAD");
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].name, "OTHER");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn missing_optional_fields_default_to_empty() {
        // Old schema (only required fields) should load with defaults
        // for everything we added later.
        let p = unique_tmp();
        std::fs::write(&p, br#"[{"name":"x","file_path":"/p","view_mode":1}]"#).unwrap();
        let list = load_from(&p);
        assert_eq!(list.len(), 1);
        assert!(list[0].f_sender.is_empty());
        assert!(!list[0].auto_watch);
        let _ = std::fs::remove_file(&p);
    }
}
