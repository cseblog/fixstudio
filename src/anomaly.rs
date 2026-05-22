//! Lightweight anomaly scanner for the timeline banner.
//!
//! Pure functions over `&[FixMessage]`. Surfaces three classes of issues an
//! HFT ops desk cares about at a glance:
//!
//!   1. Reject burst   — many `35=3` / `35=8` Reject ER messages in a short
//!                       window. Indicates pre-trade-risk pushback,
//!                       throttle-trip, or counterparty session issue.
//!   2. Sequence gap   — `MsgSeqNum (34)` jumps by more than one per
//!                       (sender, target) session. Indicates dropped wire
//!                       traffic or a counterparty resend miss.
//!   3. Latency spike  — first-ER latency for a recent window is > 5× the
//!                       baseline (P50) of the prior window. Indicates
//!                       venue degradation.
//!
//! `scan(&[FixMessage]) -> Vec<Anomaly>` is the single entry-point used by
//! the UI; everything else in this file is implementation detail and is
//! `#[cfg(test)]`-tested in isolation at the bottom.

use ahash::AHashMap as HashMap;

use crate::model::FixMessage;
use crate::session_health::parse_time_us;

#[derive(Clone, Debug, PartialEq)]
pub enum Severity {
    Warn,
    Critical,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Anomaly {
    /// More than `count` rejects landed within `window_secs` ending at the
    /// log's last reject timestamp.
    RejectBurst {
        count:       u32,
        window_secs: u32,
        severity:    Severity,
    },
    /// `gap` MsgSeqNum values were skipped on session `(sender, target)`
    /// between `prev_seq` and `next_seq`.
    SequenceGap {
        sender:    String,
        target:    String,
        prev_seq:  u64,
        next_seq:  u64,
        gap:       u64,
        severity:  Severity,
    },
    /// Recent-window P50 NOS→ER latency exceeded the prior-window baseline
    /// by `multiple` ×.
    LatencySpike {
        recent_p50_us: i64,
        baseline_p50_us: i64,
        multiple:      f64,
        severity:      Severity,
    },
}

const REJECT_BURST_WINDOW_SECS: u32 = 5;
const REJECT_BURST_WARN:        u32 = 5;
const REJECT_BURST_CRITICAL:    u32 = 20;
const LATENCY_SPIKE_MULTIPLE_WARN:     f64 = 5.0;
const LATENCY_SPIKE_MULTIPLE_CRITICAL: f64 = 10.0;
const LATENCY_WINDOW_MIN_SAMPLES:      usize = 10;

/// Top-level scanner. Runs the three checks and returns the union of
/// findings. Empty Vec on healthy logs.
pub fn scan(msgs: &[FixMessage]) -> Vec<Anomaly> {
    let mut out = Vec::new();
    if let Some(a) = scan_reject_burst(msgs)    { out.push(a); }
    out.extend(scan_sequence_gaps(msgs));
    if let Some(a) = scan_latency_spike(msgs)   { out.push(a); }
    out
}

// ─── 1. Reject burst ─────────────────────────────────────────────────────────

fn is_reject(m: &FixMessage) -> bool {
    // 35=3 → Reject (session reject)
    // 35=8 + 150=8 → ExecutionReport Reject
    // 35=9 → OrderCancelReject
    let mt = m.msg_type_raw.as_str();
    if mt == "3" || mt == "9" { return true; }
    if mt == "8" {
        return m.fields.iter().any(|f| {
            f.tag == 150 && f.value_in(&m.arena) == "8"
        });
    }
    false
}

fn scan_reject_burst(msgs: &[FixMessage]) -> Option<Anomaly> {
    // Collect (us, _) timestamps of rejects.
    let mut times: Vec<i64> = msgs.iter()
        .filter(|m| is_reject(m))
        .filter_map(|m| parse_time_us(&m.time))
        .collect();
    if times.len() < REJECT_BURST_WARN as usize { return None; }
    times.sort_unstable();
    let window_us: i64 = (REJECT_BURST_WINDOW_SECS as i64) * 1_000_000;
    // Slide a `window_us`-wide window across the rejects; find the densest one.
    let mut best: u32 = 0;
    let mut lo = 0usize;
    for hi in 0..times.len() {
        while times[hi] - times[lo] > window_us { lo += 1; }
        let count = (hi - lo + 1) as u32;
        if count > best { best = count; }
    }
    if best < REJECT_BURST_WARN { return None; }
    let severity = if best >= REJECT_BURST_CRITICAL { Severity::Critical } else { Severity::Warn };
    Some(Anomaly::RejectBurst {
        count:       best,
        window_secs: REJECT_BURST_WINDOW_SECS,
        severity,
    })
}

// ─── 2. Sequence gaps ────────────────────────────────────────────────────────

fn tag_u64(m: &FixMessage, tag: u16) -> Option<u64> {
    m.fields.iter().find(|f| f.tag == tag)
        .and_then(|f| f.value_in(&m.arena).parse::<u64>().ok())
}

fn scan_sequence_gaps(msgs: &[FixMessage]) -> Vec<Anomaly> {
    // session key = (sender, target). Last-seen MsgSeqNum per session.
    let mut last: HashMap<(String, String), u64> = HashMap::default();
    let mut out: Vec<Anomaly> = Vec::new();
    for m in msgs {
        let Some(seq) = tag_u64(m, 34) else { continue };
        let key = (m.sender.to_string(), m.target.to_string());
        if let Some(&prev) = last.get(&key) {
            if seq > prev + 1 {
                let gap = seq - prev - 1;
                let severity = if gap >= 10 { Severity::Critical } else { Severity::Warn };
                out.push(Anomaly::SequenceGap {
                    sender:   key.0.clone(),
                    target:   key.1.clone(),
                    prev_seq: prev,
                    next_seq: seq,
                    gap,
                    severity,
                });
            }
        }
        last.insert(key, seq);
    }
    out
}

// ─── 3. Latency spike ────────────────────────────────────────────────────────

/// First-ER ack latency in microseconds per ClOrdID; baseline = first half
/// of the chronologically-ordered samples, recent = second half. If the
/// recent-window P50 exceeds the baseline P50 by `LATENCY_SPIKE_MULTIPLE`,
/// raise the alarm.
fn scan_latency_spike(msgs: &[FixMessage]) -> Option<Anomaly> {
    let mut nos_us: HashMap<String, i64> = HashMap::default();
    let mut acks:   Vec<i64> = Vec::new();   // ER ack latencies, time-ordered

    for m in msgs {
        let mt = m.msg_type_raw.as_str();
        if mt == "D" {
            if m.cl_ord_id.is_empty() { continue; }
            if let Some(us) = parse_time_us(&m.time) {
                nos_us.entry(m.cl_ord_id.to_string()).or_insert(us);
            }
        } else if mt == "8" {
            if m.cl_ord_id.is_empty() { continue; }
            let Some(er_us) = parse_time_us(&m.time) else { continue };
            // Take only the FIRST ER for each NOS as the "ack".
            let Some(nos) = nos_us.get(m.cl_ord_id.as_str()).copied() else { continue };
            let lat = er_us - nos;
            if lat < 0 { continue; }
            // Remove so subsequent ERs (fills, etc.) don't double-count.
            nos_us.remove(m.cl_ord_id.as_str());
            acks.push(lat);
        }
    }
    if acks.len() < 2 * LATENCY_WINDOW_MIN_SAMPLES { return None; }

    let half = acks.len() / 2;
    let baseline_p50 = p50(&acks[..half]);
    let recent_p50   = p50(&acks[half..]);
    if baseline_p50 == 0 { return None; }
    let mult = recent_p50 as f64 / baseline_p50 as f64;
    if mult < LATENCY_SPIKE_MULTIPLE_WARN { return None; }
    let severity = if mult >= LATENCY_SPIKE_MULTIPLE_CRITICAL {
        Severity::Critical
    } else {
        Severity::Warn
    };
    Some(Anomaly::LatencySpike {
        recent_p50_us:   recent_p50,
        baseline_p50_us: baseline_p50,
        multiple:        mult,
        severity,
    })
}

fn p50(s: &[i64]) -> i64 {
    if s.is_empty() { return 0; }
    let mut v = s.to_vec();
    v.sort_unstable();
    v[v.len() / 2]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_all;

    fn parse(s: &str) -> Vec<FixMessage> { parse_all(s) }

    #[test]
    fn empty_input_returns_no_anomalies() {
        assert!(scan(&[]).is_empty());
    }

    #[test]
    fn reject_burst_triggers_when_threshold_crossed() {
        // 6 rejects in 1 second → above WARN(5) but below CRITICAL(20).
        let mut raw = String::new();
        for i in 0..6 {
            raw.push_str(&format!(
                "8=FIX.4.4|9=1|35=3|49=A|56=B|34={i}|52=20240101-09:00:00.{:03}|10=000|",
                i * 10,
            ));
        }
        let msgs = parse(&raw);
        let mut got_burst = false;
        for a in scan_reject_burst(&msgs) {
            match a {
                Anomaly::RejectBurst { count, severity, .. } => {
                    got_burst = true;
                    assert!(count >= 5);
                    assert_eq!(severity, Severity::Warn);
                }
                _ => unreachable!(),
            }
        }
        assert!(got_burst, "expected RejectBurst");
    }

    #[test]
    fn sequence_gap_detected() {
        let raw = concat!(
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=1|52=20240101-09:00:00.000|11=O1|10=000|",
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=5|52=20240101-09:00:00.001|11=O2|10=000|",
        );
        let msgs = parse(raw);
        let gaps = scan_sequence_gaps(&msgs);
        assert_eq!(gaps.len(), 1);
        if let Anomaly::SequenceGap { gap, prev_seq, next_seq, .. } = &gaps[0] {
            assert_eq!(*gap, 3);
            assert_eq!(*prev_seq, 1);
            assert_eq!(*next_seq, 5);
        } else {
            panic!("expected SequenceGap");
        }
    }

    #[test]
    fn sequence_gap_only_within_same_session() {
        // Two sessions, each seq=1 then seq=2 — no gap.
        let raw = concat!(
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=1|52=20240101-09:00:00.000|11=O1|10=000|",
            "8=FIX.4.4|9=1|35=D|49=C|56=D|34=1|52=20240101-09:00:00.001|11=O2|10=000|",
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=2|52=20240101-09:00:00.002|11=O3|10=000|",
            "8=FIX.4.4|9=1|35=D|49=C|56=D|34=2|52=20240101-09:00:00.003|11=O4|10=000|",
        );
        let msgs = parse(raw);
        assert!(scan_sequence_gaps(&msgs).is_empty());
    }

    #[test]
    fn latency_spike_needs_min_samples() {
        // Only 4 ack pairs total → below 2*MIN(10)=20, no scan.
        let mut raw = String::new();
        for i in 0..4 {
            raw.push_str(&format!(
                "8=FIX.4.4|9=1|35=D|49=A|56=B|34={}|52=20240101-09:00:00.000|11=O{i}|10=000|",
                i*2,
            ));
            raw.push_str(&format!(
                "8=FIX.4.4|9=1|35=8|49=B|56=A|34={}|52=20240101-09:00:00.001|11=O{i}|150=2|39=0|10=000|",
                i*2+1,
            ));
        }
        let msgs = parse(&raw);
        assert!(scan_latency_spike(&msgs).is_none());
    }

    #[test]
    fn no_alerts_on_clean_session() {
        let raw = concat!(
            "8=FIX.4.4|9=1|35=A|49=A|56=B|34=1|52=20240101-09:00:00.000|10=000|",
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=2|52=20240101-09:00:00.001|11=O1|10=000|",
            "8=FIX.4.4|9=1|35=8|49=B|56=A|34=3|52=20240101-09:00:00.002|11=O1|150=2|39=2|10=000|",
        );
        assert!(scan(&parse(raw)).is_empty());
    }
}
