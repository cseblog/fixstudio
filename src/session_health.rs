//! Session health diagnostics — rule-based detection of FIX session anomalies.

use std::collections::HashMap;

use crate::model::FixMessage;

// ── Tag helper ────────────────────────────────────────────────────────────────

fn tag_val<'a>(msg: &'a FixMessage, tag: u16) -> &'a str {
    msg.fields
        .iter()
        .find(|f| f.tag == tag)
        .map(|f| f.value.as_str())
        .unwrap_or("")
}

// ── Time parsing ──────────────────────────────────────────────────────────────

pub fn parse_time_us(s: &str) -> Option<i64> {
    let time_part: &str = if let Some(sp) = s.find(' ') {
        &s[sp + 1..]
    } else if let Some(dash) = s.find('-') {
        &s[dash + 1..]
    } else {
        return None;
    };
    let (hms, frac_opt) = match time_part.find('.') {
        Some(dot) => (&time_part[..dot], Some(&time_part[dot + 1..])),
        None      => (time_part, None),
    };
    let mut parts = hms.split(':');
    let h: i64   = parts.next()?.parse().ok()?;
    let m: i64   = parts.next()?.parse().ok()?;
    let sec: i64 = parts.next()?.parse().ok()?;
    let mut us   = (h * 3_600 + m * 60 + sec) * 1_000_000;
    if let Some(frac) = frac_opt {
        let flen   = frac.len().min(6);
        let fval: i64 = frac[..flen].parse().unwrap_or(0);
        us += fval * 10i64.pow((6 - flen) as u32);
    }
    Some(us)
}

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Debug)]
#[allow(dead_code)]
pub enum IssueSeverity {
    Critical,
    Warning,
    Info,
}

#[derive(Clone, PartialEq, Debug)]
pub enum HealthIssueKind {
    HeartbeatGap,
    SequenceGap,
    ExcessiveResends,
    Reconnect,
    MessageRateBurst,
    LateCancel,
    RejectedCancel,
}

#[derive(Clone, PartialEq)]
pub struct HealthIssue {
    pub kind:            HealthIssueKind,
    pub severity:        IssueSeverity,
    pub time:            String,
    pub msg_indices:     Vec<usize>,
    pub technical_desc:  String,
    pub business_impact: String,
}

#[derive(Clone, PartialEq)]
pub struct SessionHealthReport {
    pub issues: Vec<HealthIssue>,
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run_health_checks(messages: &[FixMessage]) -> SessionHealthReport {
    let mut issues: Vec<HealthIssue> = Vec::new();
    detect_sequence_gaps(messages, &mut issues);
    detect_reconnects(messages, &mut issues);
    detect_heartbeat_gaps(messages, &mut issues);
    detect_excessive_resends(messages, &mut issues);
    detect_message_rate_bursts(messages, &mut issues);
    detect_late_cancels(messages, &mut issues);
    detect_rejected_cancels(messages, &mut issues);
    SessionHealthReport { issues }
}

// ── Rule: sequence gaps ───────────────────────────────────────────────────────

fn detect_sequence_gaps(messages: &[FixMessage], issues: &mut Vec<HealthIssue>) {
    let mut last_seq: u64  = 0;
    let mut last_idx: usize = 0;
    for i in 0..messages.len() {
        let msg      = &messages[i];
        let seq_str  = tag_val(msg, 34);
        let msg_type = tag_val(msg, 35);
        if seq_str.is_empty()  { continue; }
        if msg_type == "2"     { continue; } // ResendRequest — skip
        let Ok(seq) = seq_str.parse::<u64>() else { continue };
        if last_seq > 0 && seq > last_seq + 1 {
            let missing = seq - last_seq - 1;
            issues.push(HealthIssue {
                kind:            HealthIssueKind::SequenceGap,
                severity:        IssueSeverity::Warning,
                time:            tag_val(msg, 52).to_string(),
                msg_indices:     vec![last_idx, i],
                technical_desc:  format!(
                    "Sequence gap: MsgSeqNum {last_seq} → {seq} ({missing} missing)"
                ),
                business_impact: "Missing messages may indicate dropped packets or a session \
                    reset. Verify order state with counterparty.".to_string(),
            });
        }
        last_seq = seq;
        last_idx = i;
    }
}

// ── Rule: reconnects ──────────────────────────────────────────────────────────

fn detect_reconnects(messages: &[FixMessage], issues: &mut Vec<HealthIssue>) {
    let mut logons: HashMap<(String, String), Vec<(String, usize)>> = HashMap::new();
    for i in 0..messages.len() {
        let msg = &messages[i];
        if tag_val(msg, 35) != "A" { continue; }
        let sender = tag_val(msg, 49).to_string();
        let target = tag_val(msg, 56).to_string();
        let time   = tag_val(msg, 52).to_string();
        logons.entry((sender, target)).or_default().push((time, i));
    }
    for ((sender, target), entries) in &logons {
        if entries.len() < 2 { continue; }
        let reconnect_count = entries.len() - 1;
        let indices: Vec<usize> = entries.iter().map(|(_, i)| *i).collect();
        issues.push(HealthIssue {
            kind:            HealthIssueKind::Reconnect,
            severity:        IssueSeverity::Warning,
            time:            entries[1].0.clone(),
            msg_indices:     indices,
            technical_desc:  format!(
                "{reconnect_count} reconnect(s) for {sender} → {target} \
                ({} Logon messages)", entries.len()
            ),
            business_impact: "Multiple logons indicate the session was interrupted. \
                Check for network instability or gateway restarts. \
                Verify no orders were lost during reconnects.".to_string(),
        });
    }
}

// ── Rule: heartbeat gaps ──────────────────────────────────────────────────────

fn detect_heartbeat_gaps(messages: &[FixMessage], issues: &mut Vec<HealthIssue>) {
    let heartbeat_interval: i64 = messages
        .iter()
        .find(|m| tag_val(m, 35) == "A")
        .and_then(|m| tag_val(m, 108).parse::<i64>().ok())
        .unwrap_or(30);
    let threshold_us = heartbeat_interval * 1_500_000; // 1.5× interval

    let heartbeats: Vec<(i64, usize)> = (0..messages.len())
        .filter(|&i| tag_val(&messages[i], 35) == "0")
        .filter_map(|i| Some((parse_time_us(tag_val(&messages[i], 52))?, i)))
        .collect();

    for window in heartbeats.windows(2) {
        let (prev_us, _) = window[0];
        let (curr_us, curr_idx) = window[1];
        if curr_us - prev_us > threshold_us {
            let gap_ms = (curr_us - prev_us) / 1_000;
            issues.push(HealthIssue {
                kind:            HealthIssueKind::HeartbeatGap,
                severity:        IssueSeverity::Warning,
                time:            tag_val(&messages[curr_idx], 52).to_string(),
                msg_indices:     vec![curr_idx],
                technical_desc:  format!(
                    "Heartbeat gap of {gap_ms}ms (expected ≤{}ms)",
                    heartbeat_interval * 1_500
                ),
                business_impact: "A missed heartbeat may indicate TCP issues or \
                    high gateway load. The counterparty may have sent a TestRequest. \
                    Check network health around this time.".to_string(),
            });
        }
    }
}

// ── Rule: excessive resends ───────────────────────────────────────────────────

fn detect_excessive_resends(messages: &[FixMessage], issues: &mut Vec<HealthIssue>) {
    const RESEND_THRESHOLD_PER_1000: usize = 5;
    let total = messages.len();
    if total == 0 { return; }
    let resend_count = (0..total)
        .filter(|&i| tag_val(&messages[i], 35) == "2")
        .count();
    let rate_per_1000 = resend_count * 1_000 / total;
    if rate_per_1000 <= RESEND_THRESHOLD_PER_1000 { return; }
    issues.push(HealthIssue {
        kind:            HealthIssueKind::ExcessiveResends,
        severity:        IssueSeverity::Warning,
        time:            String::new(),
        msg_indices:     Vec::new(),
        technical_desc:  format!(
            "{resend_count} ResendRequests in {total} messages ({rate_per_1000}/1000)"
        ),
        business_impact: "High resend rate indicates persistent message loss. \
            This can cause order duplication if not handled correctly. \
            Review your sequence-number management and recovery logic.".to_string(),
    });
}

// ── Rule: message rate bursts ─────────────────────────────────────────────────

fn detect_message_rate_bursts(messages: &[FixMessage], issues: &mut Vec<HealthIssue>) {
    const BURST_THRESHOLD: usize = 100;
    let timed: Vec<(i64, usize)> = (0..messages.len())
        .filter_map(|i| Some((parse_time_us(tag_val(&messages[i], 52))?, i)))
        .collect();
    if timed.is_empty() { return; }

    let mut window_start = 0;
    let mut last_burst_end: usize = 0;
    for end in 0..timed.len() {
        while timed[end].0 - timed[window_start].0 > 1_000_000 {
            window_start += 1;
        }
        let window_count = end - window_start + 1;
        if window_count <= BURST_THRESHOLD { continue; }
        if end <= last_burst_end           { continue; }
        let (_, burst_idx) = timed[end];
        issues.push(HealthIssue {
            kind:            HealthIssueKind::MessageRateBurst,
            severity:        IssueSeverity::Info,
            time:            tag_val(&messages[burst_idx], 52).to_string(),
            msg_indices:     vec![burst_idx],
            technical_desc:  format!("{window_count} messages in 1 second"),
            business_impact: "High message rate may cause queuing at the gateway. \
                Consider rate-limiting or spreading orders over time to avoid \
                congestion.".to_string(),
        });
        last_burst_end = end + BURST_THRESHOLD;
    }
}

// ── Rule: late cancels ────────────────────────────────────────────────────────

fn detect_late_cancels(messages: &[FixMessage], issues: &mut Vec<HealthIssue>) {
    // Find the latest fill time per ClOrdID.
    let mut last_fill_us: HashMap<String, i64> = HashMap::new();
    for msg in messages.iter() {
        if tag_val(msg, 35) != "8"  { continue; }
        let exec_type = tag_val(msg, 150);
        let ord_status = tag_val(msg, 39);
        if exec_type != "F" && ord_status != "2" { continue; }
        let cl_ord_id = tag_val(msg, 11).to_string();
        if cl_ord_id.is_empty() { continue; }
        let Some(us) = parse_time_us(tag_val(msg, 52)) else { continue };
        let entry = last_fill_us.entry(cl_ord_id).or_insert(i64::MIN);
        if us > *entry { *entry = us; }
    }

    for (i, msg) in messages.iter().enumerate() {
        if tag_val(msg, 35) != "F" { continue; }
        let orig_id = tag_val(msg, 41).to_string();
        if orig_id.is_empty() { continue; }
        let Some(cancel_us) = parse_time_us(tag_val(msg, 52)) else { continue };
        let Some(&fill_us) = last_fill_us.get(&orig_id) else { continue };
        if fill_us == i64::MIN || cancel_us <= fill_us { continue; }
        issues.push(HealthIssue {
            kind:            HealthIssueKind::LateCancel,
            severity:        IssueSeverity::Warning,
            time:            tag_val(msg, 52).to_string(),
            msg_indices:     vec![i],
            technical_desc:  format!(
                "OrderCancelRequest for {orig_id} arrived after fill"
            ),
            business_impact: "A cancel sent after fill indicates a race condition. \
                The cancel was ineffective. Review your cancel logic and check \
                execution status before sending cancel requests.".to_string(),
        });
    }
}

// ── Rule: rejected cancels ────────────────────────────────────────────────────

fn detect_rejected_cancels(messages: &[FixMessage], issues: &mut Vec<HealthIssue>) {
    let reject_indices: Vec<usize> = (0..messages.len())
        .filter(|&i| tag_val(&messages[i], 35) == "9")
        .collect();
    if reject_indices.is_empty() { return; }
    let first_time = tag_val(&messages[reject_indices[0]], 52).to_string();
    let count = reject_indices.len();
    issues.push(HealthIssue {
        kind:            HealthIssueKind::RejectedCancel,
        severity:        IssueSeverity::Warning,
        time:            first_time,
        msg_indices:     reject_indices,
        technical_desc:  format!("{count} OrderCancelReject message(s)"),
        business_impact: "Rejected cancels mean some orders may still be live despite \
            cancel requests. Investigate the reject reason (tag 102) and update \
            your OMS to reflect the correct live order state.".to_string(),
    });
}
