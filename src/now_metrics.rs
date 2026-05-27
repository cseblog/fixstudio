//! Snapshot metrics for the "Now" dashboard.
//!
//! All inputs are pure — no clocks, no signals. The dashboard treats the
//! latest timestamp seen in the log as "now", so replayed historical
//! logs produce a meaningful single-screen view of the last N seconds.
//!
//! One `compute(...)` call walks the messages twice: once to find the
//! reference clock, once to fold into the snapshot. Cost is O(N) on the
//! full log, which is fine for typical files (we already pay this on
//! load) and acceptable for live-tail re-runs since the snapshot only
//! re-renders when the message Signal flips.

use ahash::AHashMap as HashMap;

use crate::model::FixMessage;
use crate::session_health::parse_time_us;

const WINDOW_SECS: i64 = 30;

#[derive(Clone, Debug, PartialEq)]
pub struct LpShare {
    pub lp:    String,
    pub count: u64,
    pub pct:   f64,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct NowSnapshot {
    /// Latest timestamp seen in the log, rendered for display.
    pub now_label:         String,
    /// Width of the rolling window in seconds.
    pub window_secs:       i64,

    // ── Rolling window ──────────────────────────────────────────────────
    pub window_messages:   u64,
    pub window_rejects:    u64,
    pub window_reject_pct: f64,
    pub window_ack_p50_ms: f64,
    pub window_ack_p95_ms: f64,
    pub window_ack_count:  u64,

    // ── Cumulative (full log) ───────────────────────────────────────────
    pub open_orders:       u64,
    pub total_messages:    u64,

    // ── Per-LP share within window ──────────────────────────────────────
    pub top_lps:           Vec<LpShare>,
}

/// Build a fresh snapshot. Empty input yields a zeroed snapshot —
/// caller decides how to render the empty state.
pub fn compute(msgs: &[FixMessage]) -> NowSnapshot {
    if msgs.is_empty() {
        return NowSnapshot { window_secs: WINDOW_SECS, ..Default::default() };
    }

    // Single pass to find the latest time — needed before any windowing.
    let now_us = msgs.iter()
        .filter_map(|m| parse_time_us(&m.time))
        .max()
        .unwrap_or(0);
    let window_start = now_us - WINDOW_SECS * 1_000_000;

    let now_label = msgs.iter()
        .filter(|m| parse_time_us(&m.time) == Some(now_us))
        .map(|m| m.time.to_string())
        .next()
        .unwrap_or_default();

    // Fold counters in a single pass over the full log.
    let mut window_messages = 0u64;
    let mut window_rejects  = 0u64;
    let mut lp_counts: HashMap<String, u64> = HashMap::default();

    // Order state for "open orders" — final-status counts.
    // ord_status final = 2 (filled), 4 (cancelled), 8 (rejected).
    let mut order_total = 0u64;
    let mut order_final: HashMap<String, u8> = HashMap::default();

    // Ack-latency samples within window.
    let mut nos_us: HashMap<String, i64> = HashMap::default();
    let mut ack_latencies_window: Vec<i64> = Vec::new();

    for m in msgs {
        let mt = m.msg_type_raw.as_str();
        let t  = parse_time_us(&m.time).unwrap_or(0);
        let in_window = t >= window_start;

        if in_window {
            window_messages += 1;
            // LP = whichever side isn't us. We don't model "us" so use the
            // sender as the LP proxy — same convention the LP scorecard uses.
            // Skip session-layer noise (Heartbeat / TestRequest) from share.
            if mt != "0" && mt != "1" && !m.sender.is_empty() {
                *lp_counts.entry(m.sender.to_string()).or_insert(0) += 1;
            }
        }

        // Order accounting (full log, not window — open orders is a
        // cumulative running balance).
        if mt == "D" {
            order_total += 1;
        }
        if mt == "8" {
            let cl = m.cl_ord_id.as_str();
            if !cl.is_empty() {
                if let Some(f) = m.fields.iter().find(|f| f.tag == 39) {
                    let v = f.value_in(&m.arena);
                    if v == "2" || v == "4" || v == "8" {
                        order_final.insert(cl.to_string(), 1);
                    }
                }
            }
        }

        // Reject classification (window only).
        if in_window && is_reject(m) {
            window_rejects += 1;
        }

        // Latency: NOS time → first ER time. Within-window means the ER
        // landed within the window; the NOS could be older.
        if mt == "D" {
            let cl = m.cl_ord_id.as_str();
            if !cl.is_empty() {
                nos_us.entry(cl.to_string()).or_insert(t);
            }
        } else if mt == "8" && in_window {
            let cl = m.cl_ord_id.as_str();
            if let Some(start) = nos_us.remove(cl) {
                let lat = t - start;
                if lat >= 0 { ack_latencies_window.push(lat); }
            }
        }
    }

    let open_orders = order_total.saturating_sub(order_final.len() as u64);

    let window_reject_pct = if window_messages > 0 {
        window_rejects as f64 / window_messages as f64 * 100.0
    } else { 0.0 };

    let (p50, p95) = percentiles(&mut ack_latencies_window);
    let window_ack_p50_ms = p50 as f64 / 1_000.0;
    let window_ack_p95_ms = p95 as f64 / 1_000.0;

    // Top 5 LPs by share, percentages off the window total (excludes hb/test).
    let lp_total_in_window: u64 = lp_counts.values().sum();
    let mut top_lps: Vec<LpShare> = lp_counts.into_iter()
        .map(|(lp, count)| LpShare {
            lp,
            count,
            pct: if lp_total_in_window > 0 {
                count as f64 / lp_total_in_window as f64 * 100.0
            } else { 0.0 },
        })
        .collect();
    top_lps.sort_by(|a, b| b.count.cmp(&a.count));
    top_lps.truncate(5);

    NowSnapshot {
        now_label,
        window_secs:       WINDOW_SECS,
        window_messages,
        window_rejects,
        window_reject_pct,
        window_ack_p50_ms,
        window_ack_p95_ms,
        window_ack_count:  ack_latencies_window.len() as u64,
        open_orders,
        total_messages:    msgs.len() as u64,
        top_lps,
    }
}

fn is_reject(m: &FixMessage) -> bool {
    let mt = m.msg_type_raw.as_str();
    if mt == "3" || mt == "9" { return true; }
    if mt == "8" {
        return m.fields.iter().any(|f| {
            f.tag == 150 && f.value_in(&m.arena) == "8"
        });
    }
    false
}

/// Compute (p50, p95) in microseconds. Sorts the input in place.
fn percentiles(vals: &mut [i64]) -> (i64, i64) {
    if vals.is_empty() { return (0, 0); }
    vals.sort_unstable();
    let n = vals.len();
    let pick = |pct: usize| -> i64 {
        // Nearest-rank — fine for our coarse dashboard view.
        let idx = (pct * n / 100).min(n - 1);
        vals[idx]
    };
    (pick(50), pick(95))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_all;

    #[test]
    fn empty_input_returns_default() {
        let s = compute(&[]);
        assert_eq!(s.total_messages, 0);
        assert_eq!(s.window_secs, WINDOW_SECS);
    }

    #[test]
    fn open_orders_decrement_on_fill() {
        let raw = concat!(
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=1|52=20240101-09:00:00.000|11=O1|10=000|",
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=2|52=20240101-09:00:00.001|11=O2|10=000|",
            "8=FIX.4.4|9=1|35=8|49=B|56=A|34=3|52=20240101-09:00:00.002|11=O1|150=F|39=2|10=000|",
        );
        let s = compute(&parse_all(raw));
        assert_eq!(s.open_orders, 1, "O1 filled, O2 still open");
    }

    #[test]
    fn rejects_counted_in_window() {
        // Two rejects within the 30s window.
        let raw = concat!(
            "8=FIX.4.4|9=1|35=3|49=A|56=B|34=1|52=20240101-09:00:00.000|10=000|",
            "8=FIX.4.4|9=1|35=3|49=A|56=B|34=2|52=20240101-09:00:10.000|10=000|",
        );
        let s = compute(&parse_all(raw));
        assert_eq!(s.window_rejects, 2);
        assert!(s.window_reject_pct > 99.0);
    }

    #[test]
    fn old_messages_excluded_from_window() {
        // 1 msg way in the past, 1 msg now → only the recent one in window.
        let raw = concat!(
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=1|52=20240101-08:00:00.000|11=O1|10=000|",
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=2|52=20240101-09:00:00.000|11=O2|10=000|",
        );
        let s = compute(&parse_all(raw));
        assert_eq!(s.window_messages, 1);
    }

    #[test]
    fn lp_share_sums_to_100() {
        // Two LPs sending heartbeats → both excluded. NOS only counts the
        // taker (sender=ME). Use an ER from LP_A and LP_B.
        let raw = concat!(
            "8=FIX.4.4|9=1|35=8|49=LP_A|56=ME|34=1|52=20240101-09:00:00.000|11=O1|150=F|39=2|10=000|",
            "8=FIX.4.4|9=1|35=8|49=LP_A|56=ME|34=2|52=20240101-09:00:00.001|11=O2|150=F|39=2|10=000|",
            "8=FIX.4.4|9=1|35=8|49=LP_B|56=ME|34=3|52=20240101-09:00:00.002|11=O3|150=F|39=2|10=000|",
        );
        let s = compute(&parse_all(raw));
        let sum_pct: f64 = s.top_lps.iter().map(|l| l.pct).sum();
        assert!((sum_pct - 100.0).abs() < 0.5, "got {sum_pct}");
    }

    #[test]
    fn ack_percentiles_within_window() {
        // NOS at t=0, ERs at varying offsets, all within window.
        let raw = concat!(
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=1|52=20240101-09:00:00.000|11=O1|10=000|",
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=2|52=20240101-09:00:00.000|11=O2|10=000|",
            "8=FIX.4.4|9=1|35=8|49=B|56=A|34=3|52=20240101-09:00:00.001|11=O1|150=F|39=2|10=000|",
            "8=FIX.4.4|9=1|35=8|49=B|56=A|34=4|52=20240101-09:00:00.010|11=O2|150=F|39=2|10=000|",
        );
        let s = compute(&parse_all(raw));
        assert_eq!(s.window_ack_count, 2);
        assert!(s.window_ack_p95_ms >= s.window_ack_p50_ms);
    }
}
