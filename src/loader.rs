use std::time::Instant;

use crate::model::FixMessage;
use crate::parser::parse_all_simd_bytes;

pub struct FileLoadResult {
    pub name: String,
    pub path: String,
    pub messages: Vec<FixMessage>,
    pub parse_us: u128,
    pub is_soh: bool,
}

pub struct FolderLoadResult {
    pub folder_name: String,
    pub messages: Vec<FixMessage>,
    pub parse_us: u128,
    pub file_names: Vec<String>,
}

pub async fn pick_and_load_file() -> Option<FileLoadResult> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("FIX log", &["txt", "log", "fix"])
        .add_filter("All files", &["*"])
        .pick_file()
        .await?;
    let name = file.file_name();
    let path = file.path().to_owned();
    let t = Instant::now();
    let (messages, is_soh) = match std::fs::File::open(&path)
        .and_then(|f| unsafe { memmap2::Mmap::map(&f) })
    {
        Ok(mmap) => {
            let soh = mmap.iter().take(4096).any(|&b| b == 0x01);
            (parse_all_simd_bytes(&mmap), soh)
        }
        Err(_) => {
            let bytes = file.read().await;
            let soh = bytes.iter().take(4096).any(|&b| b == 0x01);
            (parse_all_simd_bytes(&bytes), soh)
        }
    };
    let parse_us = t.elapsed().as_micros();
    let path_str = path.to_string_lossy().into_owned();
    Some(FileLoadResult { name, path: path_str, messages, parse_us, is_soh })
}

/// Load a file directly from a known path (no picker). Used for recent-file
/// reopen flows. Returns `None` if the path no longer exists or cannot be read.
pub async fn load_file_at(path: &str) -> Option<FileLoadResult> {
    let p = std::path::PathBuf::from(path);
    if !p.exists() { return None; }
    let name = p.file_name()?.to_string_lossy().into_owned();
    let t = Instant::now();
    let (messages, is_soh) = match std::fs::File::open(&p)
        .and_then(|f| unsafe { memmap2::Mmap::map(&f) })
    {
        Ok(mmap) => {
            let soh = mmap.iter().take(4096).any(|&b| b == 0x01);
            (parse_all_simd_bytes(&mmap), soh)
        }
        Err(_) => return None,
    };
    let parse_us = t.elapsed().as_micros();
    Some(FileLoadResult { name, path: path.to_string(), messages, parse_us, is_soh })
}

pub async fn pick_and_load_folder() -> Option<FolderLoadResult> {
    let folder = rfd::AsyncFileDialog::new().pick_folder().await?;
    let root = folder.path().to_owned();
    let t = Instant::now();
    const MAX_DIRS: usize = 4_096;
    let fix_exts = ["txt", "log", "fix"];
    let mut all_msgs: Vec<FixMessage>  = Vec::new();
    let mut file_names: Vec<String>    = Vec::new();
    let mut dir_stack = vec![root.clone()];
    let mut dirs_seen = 0_usize;
    while let Some(dir) = dir_stack.pop() {
        // Assert before incrementing: dirs_seen < MAX_DIRS means we have not
        // yet consumed the last permitted slot. Firing here (not after) gives
        // a meaningful panic site rather than a silent break-past-limit.
        assert!(dirs_seen < MAX_DIRS,
            "directory scan exceeded MAX_DIRS={} at {:?}", MAX_DIRS, dir);
        dirs_seen += 1;
        let rd = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(e) => {
                eprintln!("loader: cannot read dir {:?}: {}", dir, e);
                continue;
            }
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                dir_stack.push(p);
            } else if p.extension()
                .and_then(|e| e.to_str())
                .map(|e| fix_exts.contains(&e))
                .unwrap_or(false)
            {
                let mmap = match std::fs::File::open(&p)
                    .and_then(|f| unsafe { memmap2::Mmap::map(&f) })
                {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("loader: cannot mmap {:?}: {}", p, e);
                        continue;
                    }
                };
                // Bounded marker check: FIX files start with "8=FIX" in the
                // first 64 bytes — scanning the full file wastes memory bandwidth.
                let has_fix = mmap.get(..64.min(mmap.len()))
                    .map(|head| head.windows(5).any(|w| w == b"8=FIX"))
                    .unwrap_or(false);
                if !has_fix { continue; }
                let msgs = parse_all_simd_bytes(&mmap);
                if msgs.is_empty() { continue; }
                let rel = p.strip_prefix(&root).unwrap_or(&p)
                    .to_string_lossy().into_owned();
                file_names.push(format!("{rel} ({} msgs)", msgs.len()));
                all_msgs.extend(msgs);
            }
        }
    }
    file_names.sort();
    let parse_us = t.elapsed().as_micros();
    let folder_name = root.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("folder")
        .to_string();
    Some(FolderLoadResult { folder_name, messages: all_msgs, parse_us, file_names })
}
