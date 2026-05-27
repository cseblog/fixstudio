use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

const MAX_ENTRIES: usize = 8;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct RecentEntry {
    pub path: String,
    pub name: String,
}

fn store_path() -> Option<PathBuf> {
    // Use HOME/.aifixparser/recents.json — avoids pulling another dep.
    let home = std::env::var_os("HOME")?;
    let mut p = PathBuf::from(home);
    p.push(".aifixparser");
    let _ = std::fs::create_dir_all(&p);
    p.push("recents.json");
    Some(p)
}

/// Load recents from a specific file path. Returns an empty `Vec` if the file
/// doesn't exist or is malformed — used by `load()` and by tests.
pub fn load_from(path: &Path) -> Vec<RecentEntry> {
    let Ok(bytes) = std::fs::read(path) else { return Vec::new() };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

/// Push a new entry at the front of the recents list stored at `path`,
/// deduping any existing entry with the same `path` field and capping the
/// list at `MAX_ENTRIES`. Returns the updated list.
#[allow(dead_code)]
pub fn push_to(path: &Path, file_path: &str, file_name: &str) -> Vec<RecentEntry> {
    let mut list = load_from(path);
    list.retain(|e| e.path != file_path);
    list.insert(0, RecentEntry { path: file_path.to_string(), name: file_name.to_string() });
    list.truncate(MAX_ENTRIES);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, serde_json::to_vec_pretty(&list).unwrap_or_default());
    list
}

pub fn load() -> Vec<RecentEntry> {
    let Some(p) = store_path() else { return Vec::new() };
    load_from(&p)
}

/// Update the recents list and persist asynchronously. Returns the new list
/// computed *in-memory* immediately so the caller can update the UI without
/// blocking on disk I/O — the JSON write is dispatched to a background thread.
/// Assume external storage may fail; we still keep the UI list in sync.
pub fn push(path: &str, name: &str) -> Vec<RecentEntry> {
    let Some(p) = store_path() else { return Vec::new() };
    let mut list = load_from(&p);
    list.retain(|e| e.path != path);
    list.insert(0, RecentEntry { path: path.to_string(), name: name.to_string() });
    list.truncate(MAX_ENTRIES);

    // Persist on a worker thread — even a small fsync on a busy disk can stall
    // the UI runtime task. We don't propagate errors: a missing recents file
    // on the next launch is acceptable.
    let bytes = serde_json::to_vec_pretty(&list).unwrap_or_default();
    std::thread::spawn(move || {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&p, &bytes);
    });

    list
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Generate a unique tempfile path per test so parallel runs don't clash.
    fn unique_tmp() -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let mut p = std::env::temp_dir();
        p.push(format!("aifixparser_recents_test_{}_{}.json",
            std::process::id(), n));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let p = unique_tmp();
        assert!(load_from(&p).is_empty());
    }

    #[test]
    fn push_creates_file_with_single_entry() {
        let p = unique_tmp();
        let list = push_to(&p, "/x/foo.fix", "foo.fix");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].path, "/x/foo.fix");
        assert_eq!(list[0].name, "foo.fix");
        // Round-trip through disk.
        let reloaded = load_from(&p);
        assert_eq!(reloaded, list);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn push_dedupes_existing_path_and_moves_to_front() {
        let p = unique_tmp();
        push_to(&p, "/a.fix", "a");
        push_to(&p, "/b.fix", "b");
        let list = push_to(&p, "/a.fix", "a");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].path, "/a.fix");
        assert_eq!(list[1].path, "/b.fix");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn push_caps_at_max_entries() {
        let p = unique_tmp();
        for i in 0..20 {
            push_to(&p, &format!("/f{i}.fix"), &format!("f{i}"));
        }
        let list = load_from(&p);
        assert_eq!(list.len(), MAX_ENTRIES);
        // Newest first: /f19.fix, /f18.fix, …
        assert_eq!(list[0].path, "/f19.fix");
        assert_eq!(list[MAX_ENTRIES - 1].path, format!("/f{}.fix", 20 - MAX_ENTRIES));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn malformed_file_treated_as_empty() {
        let p = unique_tmp();
        std::fs::write(&p, b"not json").unwrap();
        assert!(load_from(&p).is_empty());
        let _ = std::fs::remove_file(&p);
    }
}
