//! Session summary — one-page executive report over a slice of FIX messages.

use std::collections::HashMap;

use crate::model::FixMessage;
use crate::session_health::{run_health_checks, HealthIssue, HealthIssueKind, IssueSeverity};

// ── Tag helper ────────────────────────────────────────────────────────────────

fn tag_val<'a>(msg: &'a FixMessage, tag: u16) -> &'a str {
    msg.fields
        .iter()
        .find(|f| f.tag == tag)
        .map(|f| f.value.as_str())
        .unwrap_or("")
}

fn parse_time_us(s: &str) -> Option<i64> {
    crate::session_health::parse_time_us(s)
}

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
pub struct OrderStats {
    pub total:      u64,
    pub filled:     u64,
    pub cancelled:  u64,
    pub rejected:   u64,
    pub fill_pct:   f64,
    pub cancel_pct: f64,
    pub reject_pct: f64,
}

#[derive(Clone, PartialEq)]
pub struct LatencyStats {
    pub avg_ack_ms:        f64,
    pub avg_fill_ms:       f64,
    pub worst_spike_ms:    f64,
    pub worst_spike_time:  Option<String>,
    pub worst_spike_count: u64,
}

#[derive(Clone, PartialEq)]
#[allow(dead_code)]
pub enum EventSeverity {
    Warning,
    Info,
    Resolved,
}

#[derive(Clone, PartialEq)]
pub struct NotableEvent {
    pub severity:    EventSeverity,
    pub time:        String,
    pub description: String,
}

#[derive(Clone, PartialEq)]
pub struct SessionSummary {
    pub session_label:  String,
    pub begin_string:   String,
    pub sender:         String,
    pub target:         String,
    pub start_time:     String,
    pub end_time:       String,
    pub duration_str:   String,
    pub total_messages: u64,
    pub order_stats:    OrderStats,
    pub latency_stats:  LatencyStats,
    pub top_symbols:    Vec<(String, u64)>,
    pub notable_events: Vec<NotableEvent>,
    /// Full health report — already computed during summary build, re-exposed here
    /// so the UI component does not need to run health checks a second time.
    pub health:         crate::session_health::SessionHealthReport,
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn build_session_summary(messages: &[FixMessage]) -> SessionSummary {
    let (begin_string, sender, target) = identify_session(messages);
    let (start_time, end_time, duration_str) = compute_time_range(messages);
    let order_stats   = compute_order_stats(messages);
    let latency_stats = compute_latency_stats(messages);
    let top_symbols   = compute_top_symbols(messages);
    let health        = run_health_checks(messages);
    let notable_events = health_to_events(&health.issues);

    let session_label = if sender.is_empty() && target.is_empty() {
        "Unknown Session".to_string()
    } else {
        format!("{sender} → {target}  ({begin_string})")
    };

    SessionSummary {
        session_label,
        begin_string,
        sender,
        target,
        start_time,
        end_time,
        duration_str,
        total_messages: messages.len() as u64,
        order_stats,
        latency_stats,
        top_symbols,
        notable_events,
        health,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn identify_session(messages: &[FixMessage]) -> (String, String, String) {
    // Sample the first 200 messages — enough to identify the dominant session triple
    // without scanning millions of messages for metadata that won't change.
    const SAMPLE: usize = 200;
    let mut counts: HashMap<(String, String, String), u64> = HashMap::new();
    for msg in messages.iter().take(SAMPLE) {
        let key = (
            tag_val(msg, 8).to_string(),
            tag_val(msg, 49).to_string(),
            tag_val(msg, 56).to_string(),
        );
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|((bs, snd, tgt), _)| (bs, snd, tgt))
        .unwrap_or_default()
}

fn compute_time_range(messages: &[FixMessage]) -> (String, String, String) {
    let times: Vec<&str> = messages
        .iter()
        .filter(|m| !m.time.is_empty())
        .map(|m| m.time.as_str())
        .collect();
    if times.is_empty() {
        return (String::new(), String::new(), String::new());
    }
    let start = times.iter().copied().min().unwrap_or("").to_string();
    let end   = times.iter().copied().max().unwrap_or("").to_string();
    let duration = compute_duration_str(&start, &end);
    (start, end, duration)
}

fn compute_duration_str(start: &str, end: &str) -> String {
    let start_us = parse_time_us(start).unwrap_or(0);
    let end_us   = parse_time_us(end).unwrap_or(0);
    let diff_us  = end_us.saturating_sub(start_us);
    if diff_us <= 0 { return String::new(); }
    let total_secs = diff_us / 1_000_000;
    let hours = total_secs / 3_600;
    let mins  = (total_secs % 3_600) / 60;
    let secs  = total_secs % 60;
    if hours > 0 {
        format!("{hours}h {mins}m")
    } else if mins > 0 {
        format!("{mins}m {secs}s")
    } else {
        format!("{secs}s")
    }
}

fn compute_order_stats(messages: &[FixMessage]) -> OrderStats {
    let mut total: u64 = 0;

    // Collect the last OrdStatus per ClOrdID from ExecutionReports.
    let mut last_ord_status: HashMap<String, String> = HashMap::new();

    for msg in messages.iter() {
        let msg_type = tag_val(msg, 35);
        if msg_type == "D" {
            total += 1;
        }
        if msg_type != "8" { continue; }
        let cl_ord_id = tag_val(msg, 11).to_string();
        if cl_ord_id.is_empty() { continue; }
        let ord_status = tag_val(msg, 39).to_string();
        if !ord_status.is_empty() {
            last_ord_status.insert(cl_ord_id, ord_status);
        }
    }

    let mut filled    = 0_u64;
    let mut cancelled = 0_u64;
    let mut rejected  = 0_u64;
    for status in last_ord_status.values() {
        match status.as_str() {
            "2"            => filled    += 1,
            "4"            => cancelled += 1,
            "8"            => rejected  += 1,
            _              => {}
        }
    }

    let denom = total.max(1) as f64;
    OrderStats {
        total,
        filled,
        cancelled,
        rejected,
        fill_pct:   filled    as f64 / denom * 100.0,
        cancel_pct: cancelled as f64 / denom * 100.0,
        reject_pct: rejected  as f64 / denom * 100.0,
    }
}

fn compute_latency_stats(messages: &[FixMessage]) -> LatencyStats {
    // Map ClOrdID → NOS time.
    let mut nos_times: HashMap<String, i64> = HashMap::new();
    for msg in messages.iter() {
        if tag_val(msg, 35) != "D" { continue; }
        let cl_ord_id = tag_val(msg, 11).to_string();
        if cl_ord_id.is_empty() { continue; }
        let Some(us) = parse_time_us(tag_val(msg, 52)) else { continue };
        nos_times.entry(cl_ord_id).or_insert(us);
    }

    let mut ack_latencies:  Vec<i64> = Vec::new();
    let mut fill_latencies: Vec<(i64, String)> = Vec::new(); // (latency_us, time)

    // Collect ack from first ER, fill from last ExecType=F ER.
    let mut first_er_seen: HashMap<String, bool> = HashMap::new();

    for msg in messages.iter() {
        if tag_val(msg, 35) != "8" { continue; }
        let cl_ord_id = tag_val(msg, 11).to_string();
        if cl_ord_id.is_empty() { continue; }
        let Some(nos_us) = nos_times.get(&cl_ord_id).copied() else { continue };
        let Some(er_us) = parse_time_us(tag_val(msg, 52)) else { continue };
        let latency_us = er_us - nos_us;
        if latency_us < 0 { continue; }

        if !first_er_seen.contains_key(&cl_ord_id) {
            first_er_seen.insert(cl_ord_id.clone(), true);
            ack_latencies.push(latency_us);
        }

        let exec_type  = tag_val(msg, 150);
        let ord_status = tag_val(msg, 39);
        if exec_type == "F" || (exec_type == "2" && ord_status == "2") {
            let time = tag_val(msg, 52).to_string();
            fill_latencies.push((latency_us, time));
        }
    }

    let avg_ack_ms  = mean_ms(&ack_latencies);
    let avg_fill_ms = mean_ms(&fill_latencies.iter().map(|(us, _)| *us).collect::<Vec<_>>());

    let (worst_spike_ms, worst_spike_time, worst_spike_count) =
        compute_worst_spike(&ack_latencies, &fill_latencies);

    LatencyStats {
        avg_ack_ms,
        avg_fill_ms,
        worst_spike_ms,
        worst_spike_time,
        worst_spike_count,
    }
}

fn mean_ms(latencies: &[i64]) -> f64 {
    if latencies.is_empty() { return 0.0; }
    latencies.iter().sum::<i64>() as f64 / latencies.len() as f64 / 1_000.0
}

fn compute_worst_spike(
    ack_latencies: &[i64],
    fill_latencies: &[(i64, String)],
) -> (f64, Option<String>, u64) {
    if ack_latencies.is_empty() {
        return (0.0, None, 0);
    }
    let p95_us = percentile(ack_latencies, 95);
    let threshold_us = p95_us * 2;

    let spikes: Vec<_> = fill_latencies
        .iter()
        .filter(|(us, _)| *us > threshold_us)
        .collect();

    if spikes.is_empty() {
        let worst_us = ack_latencies.iter().copied().max().unwrap_or(0);
        return (worst_us as f64 / 1_000.0, None, 0);
    }

    let (worst_us, worst_time) = spikes
        .iter()
        .max_by_key(|(us, _)| *us)
        .map(|(us, t)| (*us, t.clone()))
        .unwrap();

    (worst_us as f64 / 1_000.0, Some(worst_time), spikes.len() as u64)
}

fn percentile(values: &[i64], pct: usize) -> i64 {
    assert!(!values.is_empty());
    assert!(pct <= 100);
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = (pct * sorted.len() / 100).min(sorted.len() - 1);
    sorted[index]
}

fn compute_top_symbols(messages: &[FixMessage]) -> Vec<(String, u64)> {
    let mut counts: HashMap<String, u64> = HashMap::new();
    for msg in messages.iter() {
        if msg.symbol.is_empty() { continue; }
        *counts.entry(msg.symbol.to_string()).or_insert(0) += 1;
    }
    let mut result: Vec<(String, u64)> = counts.into_iter().collect();
    result.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    result.truncate(10);
    result
}

fn health_to_events(issues: &[HealthIssue]) -> Vec<NotableEvent> {
    issues
        .iter()
        .map(|issue| NotableEvent {
            severity:    match issue.severity {
                IssueSeverity::Critical => EventSeverity::Warning,
                IssueSeverity::Warning  => EventSeverity::Warning,
                IssueSeverity::Info     => EventSeverity::Info,
            },
            time:        issue.time.clone(),
            description: format!(
                "{}  {}",
                health_issue_prefix(&issue.kind),
                issue.technical_desc
            ),
        })
        .collect()
}

fn health_issue_prefix(kind: &HealthIssueKind) -> &'static str {
    match kind {
        HealthIssueKind::HeartbeatGap      => "Heartbeat gap —",
        HealthIssueKind::SequenceGap       => "Sequence gap —",
        HealthIssueKind::ExcessiveResends  => "Excessive resends —",
        HealthIssueKind::Reconnect         => "Reconnect —",
        HealthIssueKind::MessageRateBurst  => "Rate burst —",
        HealthIssueKind::LateCancel        => "Late cancel —",
        HealthIssueKind::RejectedCancel    => "Rejected cancel —",
    }
}
