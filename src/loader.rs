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

/// Result of an incremental tail-load. Empty `messages` means the file
/// hasn't grown (or new bytes didn't form a complete message yet).
pub struct TailLoadResult {
    pub messages:   Vec<FixMessage>,
    /// Updated byte offset to stash for the next tail read. Always points
    /// at the start of the first incompletely-buffered line, so subsequent
    /// reads pick up trailing partial messages.
    pub new_offset: u64,
}

/// Read just the bytes that appended to `path` since `since_offset` and
/// parse the resulting tail. Returns None on I/O error / path missing.
///
/// Splits on '\n' to avoid handing the parser a half-finished message at
/// the end. The trailing partial line — if any — is included in the new
/// offset, so the next call sees it as new content once it completes.
pub async fn load_file_tail(path: &str, since_offset: u64) -> Option<TailLoadResult> {
    use std::io::{Read, Seek, SeekFrom};
    let p = std::path::PathBuf::from(path);
    let meta = std::fs::metadata(&p).ok()?;
    let size = meta.len();
    if size <= since_offset {
        // File was truncated or unchanged. Treat both as "no new data" —
        // truncation is a log-rotation signal handled by the caller via a
        // full reload reset (offset back to 0).
        return Some(TailLoadResult { messages: Vec::new(), new_offset: size });
    }
    let mut file = std::fs::File::open(&p).ok()?;
    file.seek(SeekFrom::Start(since_offset)).ok()?;
    let new_bytes_len = (size - since_offset) as usize;
    let mut buf = Vec::with_capacity(new_bytes_len);
    file.take(new_bytes_len as u64).read_to_end(&mut buf).ok()?;

    // Trim trailing partial line so we never hand a half-message to the parser.
    let split_at = buf.iter().rposition(|&b| b == b'\n').map(|i| i + 1).unwrap_or(0);
    if split_at == 0 {
        // No newline at all in the new bytes — wait for a full line next tick.
        return Some(TailLoadResult { messages: Vec::new(), new_offset: since_offset });
    }
    let (complete, _partial) = buf.split_at(split_at);
    let messages = parse_all_simd_bytes(complete);
    let consumed = since_offset + (split_at as u64);
    Some(TailLoadResult { messages, new_offset: consumed })
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
