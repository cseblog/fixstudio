//! Per-session heartbeat health snapshot used by the bottom status bar.
//!
//! Pure function over `&[FixMessage]`. For replay (historical) logs the
//! "now" reference is the latest timestamp seen in the log, so a long
//! gap before a clean Logout still flags as STALE. For live-tail mode
//! the caller can substitute the wall clock externally if desired.
//!
//! Output is keyed by `(sender, target)` session pair — the same shape
//! the anomaly banner and LP scorecard use.

use ahash::AHashMap as HashMap;

use crate::model::FixMessage;
use crate::session_health::parse_time_us;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HbStatus {
    Fresh,    // last msg within 2× heartbeat interval
    Stale,    // 2× – 4× interval — counterparty late
    Dead,     // >4× interval or logout seen — session over
}

#[derive(Clone, Debug, PartialEq)]
pub struct HeartbeatRow {
    pub sender:           String,
    pub target:           String,
    /// Negotiated HeartBtInt (tag 108) in seconds. Falls back to 30 if
    /// the Logon was missing or never seen.
    pub interval_secs:    u32,
    /// Microseconds since the most recent message on this session,
    /// measured against the log's overall max timestamp.
    pub last_msg_age_us:  i64,
    pub status:           HbStatus,
    /// Whether a Logout (35=5) was observed — the session is closed
    /// even if no time has passed, so the dot shouldn't show fresh.
    pub closed:           bool,
}

const DEFAULT_HB_INTERVAL_SECS: u32 = 30;

/// Walk the messages once, fold per-session: last seen time, HB interval,
/// logout flag. Then rank dead/stale first so the status bar shows the
/// urgent sessions on the left where the eye lands.
pub fn compute(msgs: &[FixMessage]) -> Vec<HeartbeatRow> {
    if msgs.is_empty() { return Vec::new(); }

    struct Agg {
        last_us:    i64,
        interval:   u32,
        closed:     bool,
    }
    let mut state: HashMap<(String, String), Agg> = HashMap::default();
    let mut max_us: i64 = i64::MIN;

    for m in msgs {
        let key = (m.sender.to_string(), m.target.to_string());
        if key.0.is_empty() || key.1.is_empty() { continue; }
        let Some(us) = parse_time_us(&m.time) else { continue };
        if us > max_us { max_us = us; }

        let agg = state.entry(key).or_insert(Agg {
            last_us:  i64::MIN,
            interval: DEFAULT_HB_INTERVAL_SECS,
            closed:   false,
        });
        if us > agg.last_us { agg.last_us = us; }

        match m.msg_type_raw.as_str() {
            "A" => {
                // Logon — pull negotiated heartbeat interval (tag 108).
                if let Some(v) = m.fields.iter().find(|f| f.tag == 108) {
                    if let Ok(n) = v.value_in(&m.arena).parse::<u32>() {
                        if n > 0 { agg.interval = n; }
                    }
                }
            }
            "5" => agg.closed = true,
            _   => {}
        }
    }

    if max_us == i64::MIN { return Vec::new(); }

    let mut rows: Vec<HeartbeatRow> = state.into_iter().map(|((sender, target), a)| {
        let age = (max_us - a.last_us).max(0);
        let interval_us = a.interval as i64 * 1_000_000;
        let status = if a.closed {
            HbStatus::Dead
        } else if age > interval_us * 4 {
            HbStatus::Dead
        } else if age > interval_us * 2 {
            HbStatus::Stale
        } else {
            HbStatus::Fresh
        };
        HeartbeatRow {
            sender,
            target,
            interval_secs:   a.interval,
            last_msg_age_us: age,
            status,
            closed: a.closed,
        }
    }).collect();

    // Worst-first ordering so the status bar leads with whatever the
    // operator needs to look at.
    rows.sort_by(|x, y| {
        let rank = |s: &HbStatus| match s {
            HbStatus::Dead  => 0,
            HbStatus::Stale => 1,
            HbStatus::Fresh => 2,
        };
        rank(&x.status).cmp(&rank(&y.status))
            .then_with(|| y.last_msg_age_us.cmp(&x.last_msg_age_us))
    });
    rows
}

/// Format an age in microseconds as `"3s"`, `"47s"`, `"2m12s"`, or `"1h05m"`.
pub fn fmt_age(us: i64) -> String {
    if us < 0 { return "0s".to_string(); }
    let secs_total = us / 1_000_000;
    if secs_total < 60 {
        return format!("{secs_total}s");
    }
    let mins = secs_total / 60;
    let s    = secs_total % 60;
    if mins < 60 {
        return format!("{mins}m{s:02}s");
    }
    let hours = mins / 60;
    let m     = mins % 60;
    format!("{hours}h{m:02}m")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_all;

    #[test]
    fn empty_input_returns_empty() {
        assert!(compute(&[]).is_empty());
    }

    #[test]
    fn fresh_session_within_one_interval() {
        // HeartBtInt = 30s. Logon at t=0, last msg at t=10s → fresh.
        let raw = concat!(
            "8=FIX.4.4|9=1|35=A|49=A|56=B|34=1|52=20240101-09:00:00.000|108=30|10=000|",
            "8=FIX.4.4|9=1|35=0|49=A|56=B|34=2|52=20240101-09:00:10.000|10=000|",
        );
        let rows = compute(&parse_all(raw));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, HbStatus::Fresh);
        assert_eq!(rows[0].interval_secs, 30);
    }

    #[test]
    fn stale_at_three_intervals() {
        // HeartBtInt = 10s. Last msg ~30s before the "now" set by another
        // session's later msg.
        let raw = concat!(
            "8=FIX.4.4|9=1|35=A|49=A|56=B|34=1|52=20240101-09:00:00.000|108=10|10=000|",
            "8=FIX.4.4|9=1|35=0|49=A|56=B|34=2|52=20240101-09:00:01.000|10=000|",
            "8=FIX.4.4|9=1|35=A|49=C|56=D|34=1|52=20240101-09:00:31.000|108=10|10=000|",
        );
        let rows = compute(&parse_all(raw));
        let a = rows.iter().find(|r| r.sender == "A").unwrap();
        assert_eq!(a.status, HbStatus::Stale);
    }

    #[test]
    fn dead_after_logout() {
        let raw = concat!(
            "8=FIX.4.4|9=1|35=A|49=A|56=B|34=1|52=20240101-09:00:00.000|108=30|10=000|",
            "8=FIX.4.4|9=1|35=5|49=A|56=B|34=2|52=20240101-09:00:05.000|10=000|",
        );
        let rows = compute(&parse_all(raw));
        assert_eq!(rows[0].status, HbStatus::Dead);
        assert!(rows[0].closed);
    }

    #[test]
    fn worst_status_sorted_first() {
        let raw = concat!(
            // Fresh session
            "8=FIX.4.4|9=1|35=A|49=A|56=B|34=1|52=20240101-09:00:00.000|108=30|10=000|",
            "8=FIX.4.4|9=1|35=0|49=A|56=B|34=2|52=20240101-09:01:00.000|10=000|",
            // Dead session — logged out long before "now"
            "8=FIX.4.4|9=1|35=A|49=C|56=D|34=1|52=20240101-08:00:00.000|108=30|10=000|",
            "8=FIX.4.4|9=1|35=5|49=C|56=D|34=2|52=20240101-08:01:00.000|10=000|",
        );
        let rows = compute(&parse_all(raw));
        assert_eq!(rows[0].status, HbStatus::Dead, "dead session should sort first");
    }

    #[test]
    fn fmt_age_short() {
        assert_eq!(fmt_age(5_000_000), "5s");
        assert_eq!(fmt_age(125_000_000), "2m05s");
        assert_eq!(fmt_age(3_725_000_000), "1h02m");
    }

    #[test]
    fn negative_age_doesnt_underflow() {
        assert_eq!(fmt_age(-1), "0s");
    }
}
