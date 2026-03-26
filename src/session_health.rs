//! Session health diagnostics — rule-based detection of FIX session anomalies.
//! Each rule produces a single typed HealthIssue that groups all events of that
//! kind and carries a rich detail payload for per-rule chart rendering.

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

// ── Severity / kind ───────────────────────────────────────────────────────────

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

// ── Per-rule detail structs ───────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
pub struct HeartbeatGapDetail {
    pub configured_interval_sec: i64,
    pub gaps: Vec<HeartbeatGap>,
}
#[derive(Clone, PartialEq)]
pub struct HeartbeatGap {
    pub time:    String,
    pub gap_ms:  i64,
    pub msg_idx: usize,
}

#[derive(Clone, PartialEq)]
pub struct SequenceGapDetail {
    pub total_missing: u64,
    pub gaps:          Vec<SequenceGap>,
}
#[derive(Clone, PartialEq)]
pub struct SequenceGap {
    pub from_seq: u64,
    pub to_seq:   u64,
    pub missing:  u64,
    pub time:     String,
    pub indices:  [usize; 2],
}

#[derive(Clone, PartialEq)]
pub struct ResendDetail {
    pub count:         usize,
    pub rate_per_1000: usize,
    pub instances:     Vec<ResendInstance>,
}
#[derive(Clone, PartialEq)]
pub struct ResendInstance {
    pub time:      String,
    pub begin_seq: u64,
    pub end_seq:   u64,
    pub msg_idx:   usize,
}

#[derive(Clone, PartialEq)]
pub struct ReconnectDetail {
    pub logons: Vec<LogonEvent>,
}
#[derive(Clone, PartialEq)]
pub struct LogonEvent {
    pub time:      String,
    pub seq_num:   u64,
    pub reset_seq: bool,   // true when MsgSeqNum == 1 (full session reset)
    pub msg_idx:   usize,
}

#[derive(Clone, PartialEq)]
pub struct RateBurstDetail {
    pub threshold:     usize,
    pub buckets:       Vec<RateBucket>,
    pub burst_indices: Vec<usize>,
}
#[derive(Clone, PartialEq)]
pub struct RateBucket {
    pub second_label: String,
    pub count:        usize,
}

#[derive(Clone, PartialEq)]
pub struct LateCancelDetail {
    pub cases: Vec<LateCancelCase>,
}
#[derive(Clone, PartialEq)]
pub struct LateCancelCase {
    pub cl_ord_id:   String,
    pub fill_time:   String,
    pub cancel_time: String,
    pub lag_ms:      i64,
    pub msg_idx:     usize,
}

#[derive(Clone, PartialEq)]
pub struct RejectedCancelDetail {
    pub rejections: Vec<CancelRejection>,
}
#[derive(Clone, PartialEq)]
pub struct CancelRejection {
    pub time:           String,
    pub orig_cl_ord_id: String,
    pub reason_code:    String,
    pub reason_text:    String,
    pub msg_idx:        usize,
}

// ── Typed detail enum ─────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
pub enum HealthDetail {
    HeartbeatGaps(HeartbeatGapDetail),
    SequenceGaps(SequenceGapDetail),
    Resends(ResendDetail),
    Reconnects(ReconnectDetail),
    RateBursts(RateBurstDetail),
    LateCancels(LateCancelDetail),
    RejectedCancels(RejectedCancelDetail),
}

// ── Public issue + report ─────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
pub struct HealthIssue {
    pub kind:            HealthIssueKind,
    pub severity:        IssueSeverity,
    pub time:            String,
    pub msg_indices:     Vec<usize>,
    pub technical_desc:  String,
    pub business_impact: String,
    pub detail:          HealthDetail,
}

#[derive(Clone, PartialEq)]
pub struct SessionHealthReport {
    pub issues: Vec<HealthIssue>,
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run_health_checks(messages: &[FixMessage]) -> SessionHealthReport {
    let mut issues: Vec<HealthIssue> = Vec::new();
    detect_sequence_gaps(messages, &mut issues);
    detect_heartbeat_gaps(messages, &mut issues);
    detect_reconnects(messages, &mut issues);
    detect_excessive_resends(messages, &mut issues);
    detect_message_rate_bursts(messages, &mut issues);
    detect_late_cancels(messages, &mut issues);
    detect_rejected_cancels(messages, &mut issues);
    SessionHealthReport { issues }
}

// ── Rule 1: Sequence gaps ─────────────────────────────────────────────────────
// All gaps collected into ONE issue with SequenceGapDetail.

fn detect_sequence_gaps(messages: &[FixMessage], issues: &mut Vec<HealthIssue>) {
    let mut gaps: Vec<SequenceGap> = Vec::new();
    let mut last_seq: u64  = 0;
    let mut last_idx: usize = 0;

    for (i, msg) in messages.iter().enumerate() {
        let seq_str = tag_val(msg, 34);
        if seq_str.is_empty()          { continue; }
        if tag_val(msg, 35) == "2"     { continue; } // ResendRequest — skip
        let Ok(seq) = seq_str.parse::<u64>() else { continue };
        if last_seq > 0 && seq > last_seq + 1 {
            let missing = seq - last_seq - 1;
            gaps.push(SequenceGap {
                from_seq: last_seq,
                to_seq:   seq,
                missing,
                time:    tag_val(msg, 52).to_string(),
                indices: [last_idx, i],
            });
        }
        last_seq = seq;
        last_idx = i;
    }

    if gaps.is_empty() { return; }

    let total_missing: u64 = gaps.iter().map(|g| g.missing).sum();
    let first_time          = gaps[0].time.clone();
    let count               = gaps.len();
    let msg_indices: Vec<usize> = gaps.iter()
        .flat_map(|g| g.indices.iter().copied())
        .collect();

    issues.push(HealthIssue {
        kind:     HealthIssueKind::SequenceGap,
        severity: IssueSeverity::Warning,
        time:     first_time,
        msg_indices,
        technical_desc: format!(
            "{count} sequence gap{} — {total_missing} missing message{}",
            if count == 1 { "" } else { "s" },
            if total_missing == 1 { "" } else { "s" },
        ),
        business_impact: "Missing messages may indicate dropped packets or a session \
            reset. Verify order state with counterparty.".to_string(),
        detail: HealthDetail::SequenceGaps(SequenceGapDetail { total_missing, gaps }),
    });
}

// ── Rule 2: Heartbeat gaps ────────────────────────────────────────────────────
// All gaps collected into ONE issue with HeartbeatGapDetail.

fn detect_heartbeat_gaps(messages: &[FixMessage], issues: &mut Vec<HealthIssue>) {
    let heartbeat_interval: i64 = messages
        .iter()
        .find(|m| tag_val(m, 35) == "A")
        .and_then(|m| tag_val(m, 108).parse::<i64>().ok())
        .unwrap_or(30);
    let threshold_us = heartbeat_interval * 1_500_000;

    let heartbeats: Vec<(i64, usize)> = (0..messages.len())
        .filter(|&i| tag_val(&messages[i], 35) == "0")
        .filter_map(|i| Some((parse_time_us(tag_val(&messages[i], 52))?, i)))
        .collect();

    let mut gaps: Vec<HeartbeatGap> = Vec::new();
    for window in heartbeats.windows(2) {
        let (prev_us, _)        = window[0];
        let (curr_us, curr_idx) = window[1];
        if curr_us - prev_us > threshold_us {
            gaps.push(HeartbeatGap {
                time:    tag_val(&messages[curr_idx], 52).to_string(),
                gap_ms:  (curr_us - prev_us) / 1_000,
                msg_idx: curr_idx,
            });
        }
    }

    if gaps.is_empty() { return; }

    let max_gap_ms = gaps.iter().map(|g| g.gap_ms).max().unwrap_or(0);
    let first_time = gaps[0].time.clone();
    let count      = gaps.len();
    let msg_indices: Vec<usize> = gaps.iter().map(|g| g.msg_idx).collect();

    issues.push(HealthIssue {
        kind:     HealthIssueKind::HeartbeatGap,
        severity: IssueSeverity::Warning,
        time:     first_time,
        msg_indices,
        technical_desc: format!(
            "{count} heartbeat gap{} — max {max_gap_ms}ms (threshold {}ms)",
            if count == 1 { "" } else { "s" },
            heartbeat_interval * 1_500,
        ),
        business_impact: "Missed heartbeats may indicate TCP issues or high gateway load. \
            The counterparty may have sent a TestRequest. \
            Check network health around these times.".to_string(),
        detail: HealthDetail::HeartbeatGaps(HeartbeatGapDetail {
            configured_interval_sec: heartbeat_interval,
            gaps,
        }),
    });
}

// ── Rule 3: Reconnects ────────────────────────────────────────────────────────
// One issue per (sender, target) pair with ordered LogonEvents.

fn detect_reconnects(messages: &[FixMessage], issues: &mut Vec<HealthIssue>) {
    let mut logon_map: HashMap<(String, String), Vec<LogonEvent>> = HashMap::new();

    for (i, msg) in messages.iter().enumerate() {
        if tag_val(msg, 35) != "A" { continue; }
        let sender  = tag_val(msg, 49).to_string();
        let target  = tag_val(msg, 56).to_string();
        let seq_num = tag_val(msg, 34).parse::<u64>().unwrap_or(0);
        logon_map.entry((sender, target)).or_default().push(LogonEvent {
            time:      tag_val(msg, 52).to_string(),
            seq_num,
            reset_seq: seq_num == 1,
            msg_idx:   i,
        });
    }

    for ((sender, target), mut logons) in logon_map {
        if logons.len() < 2 { continue; }
        logons.sort_by(|a, b| a.time.cmp(&b.time));
        let reconnect_count    = logons.len() - 1;
        let first_reconnect    = logons[1].time.clone();
        let msg_indices: Vec<usize> = logons.iter().map(|l| l.msg_idx).collect();

        issues.push(HealthIssue {
            kind:     HealthIssueKind::Reconnect,
            severity: IssueSeverity::Warning,
            time:     first_reconnect,
            msg_indices,
            technical_desc: format!(
                "{reconnect_count} reconnect{} — {sender} → {target} ({} Logon messages)",
                if reconnect_count == 1 { "" } else { "s" },
                logons.len(),
            ),
            business_impact: "Multiple logons indicate the session was interrupted. \
                Check for network instability or gateway restarts. \
                Verify no orders were lost during reconnects.".to_string(),
            detail: HealthDetail::Reconnects(ReconnectDetail { logons }),
        });
    }
}

// ── Rule 4: Excessive resends ─────────────────────────────────────────────────
// Single aggregated issue; collects every ResendRequest instance.

fn detect_excessive_resends(messages: &[FixMessage], issues: &mut Vec<HealthIssue>) {
    const RESEND_THRESHOLD_PER_1000: usize = 5;
    let total = messages.len();
    if total == 0 { return; }

    let instances: Vec<ResendInstance> = (0..total)
        .filter(|&i| tag_val(&messages[i], 35) == "2")
        .map(|i| {
            let msg = &messages[i];
            ResendInstance {
                time:      tag_val(msg, 52).to_string(),
                begin_seq: tag_val(msg, 7).parse::<u64>().unwrap_or(0),
                end_seq:   tag_val(msg, 16).parse::<u64>().unwrap_or(0),
                msg_idx:   i,
            }
        })
        .collect();

    let count         = instances.len();
    let rate_per_1000 = count * 1_000 / total;
    if rate_per_1000 <= RESEND_THRESHOLD_PER_1000 { return; }

    let msg_indices: Vec<usize> = instances.iter().map(|r| r.msg_idx).collect();

    issues.push(HealthIssue {
        kind:     HealthIssueKind::ExcessiveResends,
        severity: IssueSeverity::Warning,
        time:     String::new(),
        msg_indices,
        technical_desc: format!(
            "{count} ResendRequests in {total} messages ({rate_per_1000}/1000)"
        ),
        business_impact: "High resend rate indicates persistent message loss. \
            This can cause order duplication if not handled correctly. \
            Review your sequence-number management and recovery logic.".to_string(),
        detail: HealthDetail::Resends(ResendDetail { count, rate_per_1000, instances }),
    });
}

// ── Rule 5: Message rate bursts ───────────────────────────────────────────────
// O(N) bucket-by-second approach — scales to 1M messages.

fn detect_message_rate_bursts(messages: &[FixMessage], issues: &mut Vec<HealthIssue>) {
    const BURST_THRESHOLD: usize = 100;

    let mut bucket_map: HashMap<i64, usize> = HashMap::new();
    for msg in messages.iter() {
        let Some(us) = parse_time_us(tag_val(msg, 52)) else { continue };
        *bucket_map.entry(us / 1_000_000).or_insert(0) += 1;
    }
    if bucket_map.is_empty() { return; }

    let mut sorted_seconds: Vec<i64> = bucket_map.keys().copied().collect();
    sorted_seconds.sort_unstable();

    let buckets: Vec<RateBucket> = sorted_seconds.iter().map(|&sec| {
        let h = (sec / 3_600) % 24;
        let m = (sec % 3_600) / 60;
        let s = sec % 60;
        RateBucket {
            second_label: format!("{h:02}:{m:02}:{s:02}"),
            count: bucket_map[&sec],
        }
    }).collect();

    let burst_indices: Vec<usize> = buckets.iter().enumerate()
        .filter(|(_, b)| b.count > BURST_THRESHOLD)
        .map(|(i, _)| i)
        .collect();

    if burst_indices.is_empty() { return; }

    let peak            = buckets.iter().map(|b| b.count).max().unwrap_or(0);
    let first_burst_lbl = buckets[burst_indices[0]].second_label.clone();

    issues.push(HealthIssue {
        kind:     HealthIssueKind::MessageRateBurst,
        severity: IssueSeverity::Info,
        time:     first_burst_lbl,
        msg_indices: Vec::new(),
        technical_desc: format!(
            "{} burst{} exceeding {BURST_THRESHOLD} msg/sec — peak {peak}",
            burst_indices.len(),
            if burst_indices.len() == 1 { "" } else { "s" },
        ),
        business_impact: "High message rate may cause queuing at the gateway. \
            Consider rate-limiting or spreading orders over time to avoid \
            congestion.".to_string(),
        detail: HealthDetail::RateBursts(RateBurstDetail {
            threshold: BURST_THRESHOLD,
            buckets,
            burst_indices,
        }),
    });
}

// ── Rule 6: Late cancels ──────────────────────────────────────────────────────
// All late-cancel cases collected into ONE issue with LateCancelDetail.

fn detect_late_cancels(messages: &[FixMessage], issues: &mut Vec<HealthIssue>) {
    let mut last_fill_us:   HashMap<String, i64>    = HashMap::new();
    let mut last_fill_time: HashMap<String, String> = HashMap::new();

    for msg in messages.iter() {
        if tag_val(msg, 35) != "8"     { continue; }
        let exec_type  = tag_val(msg, 150);
        let ord_status = tag_val(msg, 39);
        if exec_type != "F" && ord_status != "2" { continue; }
        let cl_ord_id = tag_val(msg, 11).to_string();
        if cl_ord_id.is_empty()        { continue; }
        let Some(us) = parse_time_us(tag_val(msg, 52)) else { continue };
        let entry = last_fill_us.entry(cl_ord_id.clone()).or_insert(i64::MIN);
        if us > *entry {
            *entry = us;
            last_fill_time.insert(cl_ord_id, tag_val(msg, 52).to_string());
        }
    }

    let mut cases: Vec<LateCancelCase> = Vec::new();
    for (i, msg) in messages.iter().enumerate() {
        if tag_val(msg, 35) != "F" { continue; }
        let orig_id = tag_val(msg, 41).to_string();
        if orig_id.is_empty()      { continue; }
        let Some(cancel_us) = parse_time_us(tag_val(msg, 52)) else { continue };
        let Some(&fill_us)  = last_fill_us.get(&orig_id) else { continue };
        if fill_us == i64::MIN || cancel_us <= fill_us { continue; }
        cases.push(LateCancelCase {
            cl_ord_id:   orig_id.clone(),
            fill_time:   last_fill_time.get(&orig_id).cloned().unwrap_or_default(),
            cancel_time: tag_val(msg, 52).to_string(),
            lag_ms:      (cancel_us - fill_us) / 1_000,
            msg_idx:     i,
        });
    }

    if cases.is_empty() { return; }

    let max_lag    = cases.iter().map(|c| c.lag_ms).max().unwrap_or(0);
    let first_time = cases[0].cancel_time.clone();
    let count      = cases.len();
    let msg_indices: Vec<usize> = cases.iter().map(|c| c.msg_idx).collect();

    issues.push(HealthIssue {
        kind:     HealthIssueKind::LateCancel,
        severity: IssueSeverity::Warning,
        time:     first_time,
        msg_indices,
        technical_desc: format!(
            "{count} late cancel{} — max {max_lag}ms after fill",
            if count == 1 { "" } else { "s" },
        ),
        business_impact: "Cancels sent after fill indicate a race condition. \
            The cancels were ineffective. Review cancel logic and check \
            execution status before sending cancel requests.".to_string(),
        detail: HealthDetail::LateCancels(LateCancelDetail { cases }),
    });
}

// ── Rule 7: Rejected cancels ──────────────────────────────────────────────────
// Single aggregated issue; each rejection carries tag-102 reason.

fn detect_rejected_cancels(messages: &[FixMessage], issues: &mut Vec<HealthIssue>) {
    let rejections: Vec<CancelRejection> = (0..messages.len())
        .filter(|&i| tag_val(&messages[i], 35) == "9")
        .map(|i| {
            let msg         = &messages[i];
            let reason_code = tag_val(msg, 102).to_string();
            let reason_text = cancel_reject_reason(&reason_code).to_string();
            CancelRejection {
                time:           tag_val(msg, 52).to_string(),
                orig_cl_ord_id: tag_val(msg, 41).to_string(),
                reason_code,
                reason_text,
                msg_idx: i,
            }
        })
        .collect();

    if rejections.is_empty() { return; }

    let first_time = rejections[0].time.clone();
    let count      = rejections.len();
    let msg_indices: Vec<usize> = rejections.iter().map(|r| r.msg_idx).collect();

    issues.push(HealthIssue {
        kind:     HealthIssueKind::RejectedCancel,
        severity: IssueSeverity::Warning,
        time:     first_time,
        msg_indices,
        technical_desc: format!(
            "{count} OrderCancelReject message{}", if count == 1 { "" } else { "s" }
        ),
        business_impact: "Rejected cancels mean some orders may still be live despite \
            cancel requests. Investigate the reject reason (tag 102) and update \
            your OMS to reflect the correct live order state.".to_string(),
        detail: HealthDetail::RejectedCancels(RejectedCancelDetail { rejections }),
    });
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn cancel_reject_reason(code: &str) -> &'static str {
    match code {
        "0" => "Too Late To Cancel",
        "1" => "Unknown Order",
        "2" => "Broker Credit",
        "3" => "Already Pending Cancel",
        "4" => "Unable To Process Mass Cancel",
        _   => "Unknown Reason",
    }
}
