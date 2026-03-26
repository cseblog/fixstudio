#[derive(Clone, PartialEq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    Available(String),
    UpToDate,
}

#[derive(Clone, PartialEq)]
pub enum ViewMode {
    Timeline,
    Lifecycle,
    Overview,
    Validator,
}

pub fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse = |s: &str| -> [u32; 3] {
        let mut it = s.split('.').filter_map(|p| p.parse().ok());
        [it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0)]
    };
    parse(latest) > parse(current)
}
