#[derive(Clone, PartialEq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    Available(String),
    UpToDate,
}

#[derive(Clone, PartialEq)]
pub enum ViewMode {
    Now,
    Timeline,
    Lifecycle,
    Overview,
    Validator,
}

impl ViewMode {
    pub fn to_u8(&self) -> u8 {
        match self {
            ViewMode::Now       => 0,
            ViewMode::Timeline  => 1,
            ViewMode::Lifecycle => 2,
            ViewMode::Overview  => 3,
            ViewMode::Validator => 4,
        }
    }
    pub fn from_u8(v: u8) -> ViewMode {
        match v {
            0 => ViewMode::Now,
            2 => ViewMode::Lifecycle,
            3 => ViewMode::Overview,
            4 => ViewMode::Validator,
            _ => ViewMode::Timeline,
        }
    }
}

pub fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse = |s: &str| -> [u32; 3] {
        let mut it = s.split('.').filter_map(|p| p.parse().ok());
        [it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0)]
    };
    parse(latest) > parse(current)
}

#[cfg(test)]
mod tests {
    use super::is_newer_version;

    #[test]
    fn detects_patch_bump() {
        assert!(is_newer_version("2.0.1", "2.0.0"));
    }

    #[test]
    fn detects_minor_bump() {
        assert!(is_newer_version("2.1.0", "2.0.99"));
    }

    #[test]
    fn detects_major_bump() {
        assert!(is_newer_version("3.0.0", "2.99.99"));
    }

    #[test]
    fn rejects_same_version() {
        assert!(!is_newer_version("2.0.0", "2.0.0"));
    }

    #[test]
    fn rejects_older_version() {
        assert!(!is_newer_version("1.9.9", "2.0.0"));
    }

    #[test]
    fn handles_missing_components() {
        // "2.0" parses as [2,0,0] = "2.0.0"
        assert!(!is_newer_version("2.0", "2.0.0"));
        assert!(is_newer_version("2.0.1", "2.0"));
    }

    #[test]
    fn handles_garbage_as_zero() {
        // "abc" → [0,0,0]; "0.0.1" > [0,0,0]
        assert!(is_newer_version("0.0.1", "abc"));
    }
}
