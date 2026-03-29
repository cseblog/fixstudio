use std::time::Instant;

use crate::model::FixMessage;
use crate::parser::parse_all_simd_bytes;

pub struct FileLoadResult {
    pub name: String,
    pub messages: Vec<FixMessage>,
    pub parse_us: u64,
    pub is_soh: bool,
}

pub struct FolderLoadResult {
    pub folder_name: String,
    pub messages: Vec<FixMessage>,
    pub parse_us: u64,
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
    let parse_us = t.elapsed().as_micros() as u64;
    Some(FileLoadResult { name, messages, parse_us, is_soh })
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
        assert!(dirs_seen <= MAX_DIRS);
        dirs_seen += 1;
        if dirs_seen > MAX_DIRS { break; }
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    dir_stack.push(p);
                } else if p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| fix_exts.contains(&e))
                    .unwrap_or(false)
                {
                    let msgs = match std::fs::File::open(&p)
                        .and_then(|f| unsafe { memmap2::Mmap::map(&f) })
                    {
                        Ok(mmap) => {
                            let has_fix = mmap.windows(5).any(|w| w == b"8=FIX");
                            if !has_fix { continue; }
                            parse_all_simd_bytes(&mmap)
                        }
                        Err(_) => continue,
                    };
                    if msgs.is_empty() { continue; }
                    let rel = p.strip_prefix(&root).unwrap_or(&p)
                        .to_string_lossy().into_owned();
                    file_names.push(format!("{rel} ({} msgs)", msgs.len()));
                    all_msgs.extend(msgs);
                }
            }
        }
    }
    file_names.sort();
    let parse_us = t.elapsed().as_micros() as u64;
    let folder_name = root.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("folder")
        .to_string();
    Some(FolderLoadResult { folder_name, messages: all_msgs, parse_us, file_names })
}
