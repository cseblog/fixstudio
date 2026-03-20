//! Trade Lifecycle Reconstructor
//!
//! Groups all related FIX messages into a single timeline per order by
//! chaining: QuoteRequest (tag 131) → Quote (tag 117) → NewOrder (tag 11)
//! → ExecutionReports → CancelRequest (tag 41 OrigClOrdID) → ER(Cancelled).
//!
//! Also renders latency statistics (histogram, scatter, per-symbol breakdown).

use dioxus::prelude::*;
use dioxus::document::eval;
use std::collections::HashMap;

use crate::export::{csv_escape, now_tag};
use crate::model::FixMessage;

// ─── Helper: tag value lookup ─────────────────────────────────────────────────

fn tag_val<'a>(msg: &'a FixMessage, tag: u16) -> &'a str {
    msg.fields.iter().find(|f| f.tag == tag).map(|f| f.value.as_str()).unwrap_or("")
}

// ─── Time helpers ─────────────────────────────────────────────────────────────

fn parse_fix_time_us(s: &str) -> Option<i64> {
    let time_part: &str = if let Some(sp) = s.find(' ') {
        &s[sp + 1..]
    } else if let Some(dash) = s.find('-') {
        &s[dash + 1..]
    } else {
        return None;
    };
    let (hms_str, frac_str) = match time_part.find('.') {
        Some(dot) => (&time_part[..dot], Some(&time_part[dot + 1..])),
        None => (time_part, None),
    };
    let mut it = hms_str.split(':');
    let h: i64 = it.next()?.parse().ok()?;
    let m: i64 = it.next()?.parse().ok()?;
    let sec: i64 = it.next()?.parse().ok()?;
    let mut us: i64 = (h * 3_600 + m * 60 + sec) * 1_000_000;
    if let Some(frac) = frac_str {
        let flen = frac.len().min(6);
        let fval: i64 = frac[..flen].parse().unwrap_or(0);
        us += fval * 10i64.pow((6 - flen) as u32);
    }
    Some(us)
}

pub fn fmt_us(us: i64) -> String {
    if us < 0 { return "—".into(); }
    if us < 1_000 { format!("{}μs", us) }
    else if us < 10_000 { format!("{:.2}ms", us as f64 / 1_000.0) }
    else if us < 1_000_000 { format!("{:.1}ms", us as f64 / 1_000.0) }
    else { format!("{:.3}s", us as f64 / 1_000_000.0) }
}

fn fmt_us_short(us: i64) -> String {
    if us < 0 { return "—".into(); }
    if us < 1_000 { format!("{}μs", us) }
    else if us < 1_000_000 { format!("{:.0}ms", us as f64 / 1_000.0) }
    else { format!("{:.1}s", us as f64 / 1_000_000.0) }
}

fn latency_health(us: i64) -> &'static str {
    if us < 1_000     { "health-green"  }
    else if us < 10_000  { "health-yellow" }
    else if us < 100_000 { "health-orange" }
    else                 { "health-red"    }
}

fn cmp_opt(a: Option<i64>, b: Option<i64>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None)    => std::cmp::Ordering::Less,
        (None, Some(_))    => std::cmp::Ordering::Greater,
        (None, None)       => std::cmp::Ordering::Equal,
    }
}

fn time_to_hms(s: &str) -> String {
    if let Some(sp) = s.find(' ') { return s[sp + 1..].to_string(); }
    if let Some(d) = s.find('-') { return s[d + 1..].to_string(); }
    s.to_string()
}

// ─── Lifecycle chain data structures ─────────────────────────────────────────

#[derive(Clone, PartialEq, Debug)]
pub enum FinalStatus {
    Filled,
    PartialFill,
    Cancelled,
    Rejected,
    Expired,
    Open,
    Unknown,
}

impl FinalStatus {
    fn label(&self) -> &'static str {
        match self {
            FinalStatus::Filled      => "Filled",
            FinalStatus::PartialFill => "Partial",
            FinalStatus::Cancelled   => "Cancelled",
            FinalStatus::Rejected    => "Rejected",
            FinalStatus::Expired     => "Expired",
            FinalStatus::Open        => "Open",
            FinalStatus::Unknown     => "Unknown",
        }
    }
    fn css_class(&self) -> &'static str {
        match self {
            FinalStatus::Filled      => "status-filled",
            FinalStatus::PartialFill => "status-partial",
            FinalStatus::Cancelled   => "status-cancelled",
            FinalStatus::Rejected    => "status-rejected",
            FinalStatus::Expired     => "status-expired",
            FinalStatus::Open        => "status-open",
            FinalStatus::Unknown     => "status-unknown",
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct LifecycleChain {
    pub chain_id: String,
    pub quote_req_id: Option<String>,
    pub quote_id: Option<String>,
    pub primary_cl_ord_id: Option<String>,
    pub all_cl_ord_ids: Vec<String>,
    pub symbol: String,
    pub side: String,
    pub first_time_us: i64,
    pub last_time_us: i64,
    // Latency breakdown
    pub rfq_to_quote_us: Option<i64>,
    pub quote_to_nos_us: Option<i64>,
    pub nos_to_ack_us: Option<i64>,
    pub nos_to_fill_us: Option<i64>,
    pub total_us: i64,
    pub final_status: FinalStatus,
    pub has_rfq: bool,
    pub msg_count: usize,
    pub msg_indices: Vec<usize>,
}

// ─── Chain reconstruction ─────────────────────────────────────────────────────

/// Build a complete LifecycleChain from a set of message indices.
fn make_chain(
    messages:      &[FixMessage],
    mut indices:   Vec<usize>,
    qreq_id:       Option<String>,
    quote_id:      Option<String>,
    nos_cl_ord_ids: Vec<String>,
) -> LifecycleChain {
    // Sort by timestamp, stable-fallback to file order
    indices.sort_by_key(|&i| parse_fix_time_us(&messages[i].time).unwrap_or(i as i64));
    indices.dedup();

    let msg_count = indices.len();
    let first_time_us = indices.first()
        .and_then(|&i| parse_fix_time_us(&messages[i].time)).unwrap_or(0);
    let last_time_us  = indices.last()
        .and_then(|&i| parse_fix_time_us(&messages[i].time)).unwrap_or(0);

    // Derive display fields from first available message with symbol/side
    let symbol = indices.iter()
        .map(|&i| messages[i].symbol.as_str())
        .find(|s| !s.is_empty())
        .unwrap_or("").to_string();
    let side = indices.iter()
        .map(|&i| messages[i].side.as_str())
        .find(|s| !s.is_empty())
        .unwrap_or("").to_string();

    // Key timestamps for latency calculation
    let rfq_time = qreq_id.as_ref().and_then(|_|
        indices.iter().map(|&i| &messages[i])
            .find(|m| m.msg_type_raw == "R")
            .and_then(|m| parse_fix_time_us(&m.time))
    );
    let nos_time = indices.iter().map(|&i| &messages[i])
        .find(|m| m.msg_type_raw == "D")
        .and_then(|m| parse_fix_time_us(&m.time));
    let first_er_time = indices.iter().map(|&i| &messages[i])
        .find(|m| m.msg_type_raw == "8")
        .and_then(|m| parse_fix_time_us(&m.time));
    let last_er_time = indices.iter().rev().map(|&i| &messages[i])
        .find(|m| m.msg_type_raw == "8")
        .and_then(|m| parse_fix_time_us(&m.time));

    // Find the Quote (35=S) timestamp. First try messages already in this chain's indices.
    // Fallback: if the Quote wasn't linked (e.g. it lacks tag 131), find it via the QuoteID
    // (tag 117) carried by any NOS in the chain — covers non-standard Quote implementations.
    let quote_msg_time = indices.iter().map(|&i| &messages[i])
        .find(|m| m.msg_type_raw == "S")
        .and_then(|m| parse_fix_time_us(&m.time))
        .or_else(|| {
            let nos_qid = indices.iter().map(|&i| &messages[i])
                .find(|m| m.msg_type_raw == "D")
                .map(|m| tag_val(m, 117))
                .filter(|v| !v.is_empty())?;
            messages.iter()
                .find(|m| m.msg_type_raw == "S" && tag_val(m, 117) == nos_qid)
                .and_then(|m| parse_fix_time_us(&m.time))
        });

    let rfq_to_quote_us = match (rfq_time, quote_msg_time) {
        (Some(r), Some(q)) if q >= r => Some(q - r),
        _ => None,
    };
    let quote_to_nos_us = match (quote_msg_time.or(rfq_time), nos_time) {
        (Some(q), Some(n)) if n >= q => Some(n - q),
        _ => None,
    };
    let nos_to_ack_us = match (nos_time, first_er_time) {
        (Some(n), Some(e)) if e >= n => Some(e - n),
        _ => None,
    };
    let nos_to_fill_us = match (nos_time, last_er_time) {
        (Some(n), Some(e)) if e > n => Some(e - n),
        _ => None,
    };

    // Determine final status from last ExecReport
    let final_status = indices.iter().rev()
        .map(|&i| &messages[i])
        .find(|m| m.msg_type_raw == "8")
        .map(|m| {
            let ord_status = tag_val(m, 39);
            let exec_type  = tag_val(m, 150);
            let s = if !ord_status.is_empty() { ord_status } else { exec_type };
            match s {
                "2" | "F"       => FinalStatus::Filled,
                "1"             => FinalStatus::PartialFill,
                "4"             => FinalStatus::Cancelled,
                "8"             => FinalStatus::Rejected,
                "C" | "6"       => FinalStatus::Expired,
                "0" | "E" | "A" => FinalStatus::Open,
                _               => FinalStatus::Unknown,
            }
        })
        .unwrap_or(FinalStatus::Open);

    // chain_id: prefer primary ClOrdID, else QuoteReqID
    let chain_id = nos_cl_ord_ids.first()
        .cloned()
        .or_else(|| qreq_id.clone())
        .unwrap_or_else(|| format!("chain-{}", indices.first().copied().unwrap_or(0)));

    let has_rfq = qreq_id.is_some();
    let primary = nos_cl_ord_ids.first().cloned();

    LifecycleChain {
        chain_id,
        quote_req_id: qreq_id,
        quote_id,
        primary_cl_ord_id: primary,
        all_cl_ord_ids: nos_cl_ord_ids,
        symbol,
        side,
        first_time_us,
        last_time_us,
        rfq_to_quote_us,
        quote_to_nos_us,
        nos_to_ack_us,
        nos_to_fill_us,
        total_us: (last_time_us - first_time_us).max(0),
        final_status,
        has_rfq,
        msg_count,
        msg_indices: indices,
    }
}

/// Recursively collect all message indices belonging to a ClOrdID chain.
/// Follows OrigClOrdID (tag 41) links to gather cancel-replace branches.
fn collect_clord_tree(
    root: &str,
    messages: &[FixMessage],
    clord_idx: &HashMap<&str, Vec<usize>>,
    orig_idx:  &HashMap<&str, Vec<usize>>,
    assigned:  &mut Vec<bool>,
) -> (Vec<usize>, Vec<String>) {
    let mut all_indices = Vec::new();
    let mut all_cl_ids  = Vec::new();
    let mut queue       = vec![root.to_string()];
    let mut visited     = std::collections::HashSet::new();

    while let Some(cl_id) = queue.pop() {
        if !visited.insert(cl_id.clone()) { continue; }
        all_cl_ids.push(cl_id.clone());

        // All messages tagged with this ClOrdID
        if let Some(idxs) = clord_idx.get(cl_id.as_str()) {
            for &idx in idxs {
                if !assigned[idx] {
                    assigned[idx] = true;
                    all_indices.push(idx);
                }
            }
        }

        // Follow cancel/replace: any message with OrigClOrdID == cl_id
        if let Some(idxs) = orig_idx.get(cl_id.as_str()) {
            for &idx in idxs {
                if !assigned[idx] {
                    assigned[idx] = true;
                    all_indices.push(idx);
                }
                // New ClOrdID from the cancel/replace message
                let new_cl = messages[idx].cl_ord_id.as_str();
                if !new_cl.is_empty() { queue.push(new_cl.to_string()); }
            }
        }
    }
    (all_indices, all_cl_ids)
}

pub fn build_lifecycle_chains(messages: &[FixMessage]) -> Vec<LifecycleChain> {
    if messages.is_empty() { return vec![]; }

    // ── Build indexes (single pass) ────────────────────────────────────────
    // tag 131 = QuoteReqID  (in 35=R and 35=S)
    // tag 117 = QuoteID     (in 35=S and 35=D when accepting a quote)
    // tag  11 = ClOrdID
    // tag  41 = OrigClOrdID (in 35=F cancel and some 35=8)

    let mut qreq_idx:  HashMap<&str, Vec<usize>> = HashMap::new(); // QuoteReqID → indices
    let mut quote_idx: HashMap<&str, Vec<usize>> = HashMap::new(); // QuoteID    → indices (35=S only)
    let mut nos_qid:   HashMap<&str, Vec<usize>> = HashMap::new(); // QuoteID    → NOS indices (35=D with tag 117)
    let mut clord_idx: HashMap<&str, Vec<usize>> = HashMap::new(); // ClOrdID    → indices
    let mut orig_idx:  HashMap<&str, Vec<usize>> = HashMap::new(); // OrigClOrdID→ indices

    for (i, msg) in messages.iter().enumerate() {
        let mtype = msg.msg_type_raw.as_str();

        // ClOrdID (tag 11)
        if !msg.cl_ord_id.is_empty() {
            clord_idx.entry(msg.cl_ord_id.as_str()).or_default().push(i);
        }
        // OrigClOrdID (tag 41)
        let orig = tag_val(msg, 41);
        if !orig.is_empty() {
            orig_idx.entry(orig).or_default().push(i);
        }
        // QuoteReqID (tag 131) — present on 35=R and 35=S
        let qrid = tag_val(msg, 131);
        if !qrid.is_empty() {
            qreq_idx.entry(qrid).or_default().push(i);
        }
        // QuoteID (tag 117)
        let qid = tag_val(msg, 117);
        if !qid.is_empty() {
            if mtype == "S" {
                quote_idx.entry(qid).or_default().push(i);
            } else if mtype == "D" {
                nos_qid.entry(qid).or_default().push(i);
            }
        }
    }

    let mut assigned = vec![false; messages.len()];
    let mut chains   = Vec::new();

    // ── Pass 1: RFQ chains (starting from 35=R QuoteRequests) ────────────
    for (i, msg) in messages.iter().enumerate() {
        if msg.msg_type_raw != "R" || assigned[i] { continue; }

        let qrid = tag_val(msg, 131);
        if qrid.is_empty() { continue; }

        let mut chain_indices = Vec::new();
        assigned[i] = true;
        chain_indices.push(i);

        // Find all Quotes (35=S) with same QuoteReqID
        let mut found_quote_id: Option<String> = None;
        let mut nos_cl_ord_ids = Vec::new();

        if let Some(q_idxs) = qreq_idx.get(qrid) {
            for &qi in q_idxs {
                if qi == i || assigned[qi] { continue; }
                if messages[qi].msg_type_raw == "S" {
                    assigned[qi] = true;
                    chain_indices.push(qi);
                    let qid = tag_val(&messages[qi], 117);
                    if !qid.is_empty() && found_quote_id.is_none() {
                        found_quote_id = Some(qid.to_string());
                    }
                }
            }
        }

        // Fallback: if no 35=S was found via tag 131 (some implementations omit it),
        // do sequential pairing — find the nearest unassigned 35=S after this 35=R
        // that has an associated NOS (via tag 117 QuoteID). Stop at the next 35=R.
        if found_quote_id.is_none() {
            'seq_search: for j in (i + 1)..messages.len() {
                if assigned[j] { continue; }
                if messages[j].msg_type_raw == "R" { break 'seq_search; } // belongs to later RFQ
                if messages[j].msg_type_raw != "S" { continue; }
                let qid = tag_val(&messages[j], 117);
                if qid.is_empty() { continue; }
                if nos_qid.contains_key(qid) {
                    assigned[j] = true;
                    chain_indices.push(j);
                    found_quote_id = Some(qid.to_string());
                    break 'seq_search;
                }
            }
        }

        // Find NOS(s) that reference this QuoteID
        if let Some(ref qid) = found_quote_id {
            if let Some(nos_idxs) = nos_qid.get(qid.as_str()) {
                for &ni in nos_idxs {
                    if assigned[ni] { continue; }
                    if messages[ni].msg_type_raw == "D" {
                        let cl = messages[ni].cl_ord_id.as_str();
                        if cl.is_empty() { continue; }
                        let (extra, cl_ids) = collect_clord_tree(
                            cl, messages, &clord_idx, &orig_idx, &mut assigned,
                        );
                        chain_indices.extend(extra);
                        nos_cl_ord_ids.extend(cl_ids);
                    }
                }
            }
        }

        chains.push(make_chain(
            messages, chain_indices,
            Some(qrid.to_string()), found_quote_id, nos_cl_ord_ids,
        ));
    }

    // ── Pass 2: Standalone NOS chains (no RFQ) ────────────────────────────
    for (i, msg) in messages.iter().enumerate() {
        if msg.msg_type_raw != "D" || assigned[i] { continue; }

        let cl = msg.cl_ord_id.as_str();
        if cl.is_empty() { continue; }

        let (indices, cl_ids) = collect_clord_tree(
            cl, messages, &clord_idx, &orig_idx, &mut assigned,
        );
        if !indices.is_empty() {
            chains.push(make_chain(messages, indices, None, None, cl_ids));
        }
    }

    // ── Pass 3: Orphan ExecReports (no NOS found) ─────────────────────────
    {
        let mut orphan_groups: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, msg) in messages.iter().enumerate() {
            if assigned[i] || msg.msg_type_raw != "8" { continue; }
            if msg.cl_ord_id.is_empty() { continue; }
            orphan_groups.entry(msg.cl_ord_id.as_str()).or_default().push(i);
        }
        for (cl_id, idxs) in orphan_groups {
            for &idx in &idxs { assigned[idx] = true; }
            chains.push(make_chain(
                messages, idxs, None, None, vec![cl_id.to_string()],
            ));
        }
    }

    // Sort chains chronologically
    chains.sort_by_key(|c| c.first_time_us);
    chains
}

// ─── Phase latency stats ──────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
struct PhaseStats {
    count:   usize,
    min_us:  i64,
    mean_us: f64,
    p50_us:  i64,
    p95_us:  i64,
    p99_us:  i64,
    max_us:  i64,
}

fn compute_phase_stats(lats: &[i64]) -> Option<PhaseStats> {
    if lats.is_empty() { return None; }
    let mut s = lats.to_vec();
    s.sort_unstable();
    let n = s.len();
    let sum: i64 = s.iter().sum();
    let pct = |p: f64| s[((p / 100.0) * (n - 1) as f64).round() as usize];
    Some(PhaseStats {
        count:   n,
        min_us:  s[0],
        max_us:  s[n - 1],
        mean_us: sum as f64 / n as f64,
        p50_us:  pct(50.0),
        p95_us:  pct(95.0),
        p99_us:  pct(99.0),
    })
}

// ─── Flow node types ──────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
struct FlowNode {
    label:     String,
    sublabel:  String,
    time_str:  String,
    time_us:   i64,
    delta_us:  i64,
    kind:      FlowKind,
}

#[derive(Clone, PartialEq)]
enum FlowKind {
    RfqRequest,
    RfqQuote,
    NewOrder,
    ExecNew,
    ExecPartial,
    ExecFilled,
    ExecCanceled,
    ExecRejected,
    ExecExpired,
    CancelReq,
    Other,
}

impl FlowKind {
    fn color(&self) -> &'static str {
        match self {
            FlowKind::RfqRequest   => "#bd93f9",
            FlowKind::RfqQuote     => "#8be9fd",
            FlowKind::NewOrder     => "#bd93f9",
            FlowKind::ExecNew      => "#8be9fd",
            FlowKind::ExecPartial  => "#f1fa8c",
            FlowKind::ExecFilled   => "#50fa7b",
            FlowKind::ExecCanceled => "#ffb86c",
            FlowKind::ExecRejected => "#ff5555",
            FlowKind::ExecExpired  => "#6272a4",
            FlowKind::CancelReq    => "#ff79c6",
            FlowKind::Other        => "#6272a4",
        }
    }
}

// ─── Chain flow builder ───────────────────────────────────────────────────────

fn build_chain_flow(messages: &[FixMessage], chain: &LifecycleChain) -> Vec<FlowNode> {
    let mut msgs: Vec<(usize, &FixMessage)> = chain.msg_indices.iter()
        .map(|&i| (i, &messages[i]))
        .collect();
    msgs.sort_by_key(|(_, m)| parse_fix_time_us(&m.time).unwrap_or(i64::MAX));

    let mut nodes = Vec::with_capacity(msgs.len());
    let mut prev_us: Option<i64> = None;

    for (_, msg) in msgs {
        let t     = parse_fix_time_us(&msg.time).unwrap_or(0);
        let delta = prev_us.map(|p| (t - p).max(0)).unwrap_or(0);
        prev_us   = Some(t);
        let time_str = time_to_hms(&msg.time);

        let (label, sublabel, kind) = match msg.msg_type_raw.as_str() {
            "R" => {
                let sym = msg.symbol.as_str();
                let qty = tag_val(msg, 38);
                let sub = if !sym.is_empty() { format!("{} {}", sym, qty) } else { String::new() };
                ("RFQ".into(), sub, FlowKind::RfqRequest)
            }
            "S" => {
                let bid = tag_val(msg, 132);
                let off = tag_val(msg, 133);
                let sub = if !bid.is_empty() { format!("{}/{}", bid, off) } else { String::new() };
                ("Quote".into(), sub, FlowKind::RfqQuote)
            }
            "D" => {
                let sym  = msg.symbol.as_str();
                let side = msg.side.as_str();
                let qty  = tag_val(msg, 38);
                let px   = tag_val(msg, 44);
                let ord_type = match tag_val(msg, 40) {
                    "1" => "Mkt",
                    "2"       => "Lmt",
                    "D"       => "Qtd",
                    _         => "Ord",
                };
                let sub = if !sym.is_empty() { format!("{} {} {}@{}", side, sym, qty, px) } else { String::new() };
                (format!("NOS:{}", ord_type), sub, FlowKind::NewOrder)
            }
            "8" => {
                let ord_status = tag_val(msg, 39);
                let exec_type  = tag_val(msg, 150);
                let last_qty   = tag_val(msg, 32);
                let last_px    = tag_val(msg, 31);
                let cum_qty    = tag_val(msg, 14);
                let sublbl = if !last_qty.is_empty() && last_qty != "0" && !last_px.is_empty() {
                    format!("{}@{}", last_qty, last_px)
                } else if !cum_qty.is_empty() && cum_qty != "0" {
                    format!("cum:{}", cum_qty)
                } else { String::new() };
                let s = if !ord_status.is_empty() { ord_status } else { exec_type };
                let (lbl, k) = match s {
                    "0"       => ("ER:New",      FlowKind::ExecNew),
                    "1"       => ("ER:Partial",  FlowKind::ExecPartial),
                    "2" | "F" => ("ER:Filled",   FlowKind::ExecFilled),
                    "4"       => ("ER:Cancelled",FlowKind::ExecCanceled),
                    "8"       => ("ER:Rejected", FlowKind::ExecRejected),
                    "C" | "6" => ("ER:Expired",  FlowKind::ExecExpired),
                    _         => ("ExecRpt",     FlowKind::Other),
                };
                (lbl.into(), sublbl, k)
            }
            "F" | "9" => ("CancelReq".into(), String::new(), FlowKind::CancelReq),
            t => (format!("35={}", t), String::new(), FlowKind::Other),
        };

        nodes.push(FlowNode { label, sublabel, time_str, time_us: t, delta_us: delta, kind });
    }
    nodes
}

// ─── Inline chain timeline ────────────────────────────────────────────────────

#[derive(Clone)]
struct TLSeg {
    delta_us:  i64,
    label:     String,
    sublabel:  String,
    color:     &'static str,
    first:     bool,   // no arrow prefix
    is_cancel: bool,   // show └─ prefix
}

#[derive(Clone)]
struct TLLine {
    indent: usize,   // "node+arrow" segments to skip at start
    segs:   Vec<TLSeg>,
}

fn build_timeline_lines(nodes: &[FlowNode]) -> Vec<TLLine> {
    let mut preamble: Vec<&FlowNode> = Vec::new();
    let mut er_nodes: Vec<&FlowNode> = Vec::new();
    let mut cancel:   Vec<&FlowNode> = Vec::new();
    let mut in_cancel = false;

    for n in nodes {
        match &n.kind {
            FlowKind::RfqRequest | FlowKind::RfqQuote | FlowKind::NewOrder | FlowKind::Other => {
                preamble.push(n);
            }
            FlowKind::CancelReq => { in_cancel = true; cancel.push(n); }
            _ => { if in_cancel { cancel.push(n); } else { er_nodes.push(n); } }
        }
    }

    let pre = preamble.len();
    let mut lines: Vec<TLLine> = Vec::new();

    // Line 0: preamble nodes + first ER
    {
        let mut segs: Vec<TLSeg> = Vec::new();
        for (i, n) in preamble.iter().enumerate() {
            segs.push(TLSeg { delta_us: n.delta_us, label: n.label.clone(),
                sublabel: n.sublabel.clone(), color: n.kind.color(),
                first: i == 0, is_cancel: false });
        }
        if let Some(er0) = er_nodes.first() {
            segs.push(TLSeg { delta_us: er0.delta_us, label: er0.label.clone(),
                sublabel: er0.sublabel.clone(), color: er0.kind.color(),
                first: segs.is_empty(), is_cancel: false });
        }
        if !segs.is_empty() { lines.push(TLLine { indent: 0, segs }); }
    }

    // Additional ER lines branch from after preamble
    for er in er_nodes.iter().skip(1) {
        lines.push(TLLine {
            indent: pre,
            segs: vec![TLSeg { delta_us: er.delta_us, label: er.label.clone(),
                sublabel: er.sublabel.clone(), color: er.kind.color(),
                first: false, is_cancel: false }],
        });
    }

    // Cancel chain branches from preamble + 1 ER column
    if !cancel.is_empty() {
        let segs = cancel.iter().enumerate().map(|(i, n)|
            TLSeg { delta_us: n.delta_us, label: n.label.clone(),
                sublabel: n.sublabel.clone(), color: n.kind.color(),
                first: i == 0, is_cancel: i == 0 }
        ).collect();
        lines.push(TLLine { indent: pre + 1, segs });
    }

    lines
}

// ─── Component ────────────────────────────────────────────────────────────────

const PAGE_SIZE: usize = 100;

fn chains_to_csv(chains: &[LifecycleChain]) -> String {
    let opt = |v: Option<i64>| v.map(|n| n.to_string()).unwrap_or_default();
    let mut out = String::with_capacity(chains.len() * 120);
    out.push_str(
        "ChainID,Symbol,Side,Status,FirstTime_us,\
         RFQ_Quote_us,Quote_NOS_us,NOS_ER_us,ER_Fill_us,Duration_us,MsgCount\n"
    );
    for c in chains {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{}\n",
            csv_escape(&c.chain_id),
            csv_escape(&c.symbol),
            csv_escape(&c.side),
            c.final_status.label(),
            c.first_time_us,
            opt(c.rfq_to_quote_us),
            opt(c.quote_to_nos_us),
            opt(c.nos_to_ack_us),
            opt(c.nos_to_fill_us),
            c.total_us,
            c.msg_count,
        ));
    }
    out
}

#[derive(Clone, Copy, PartialEq)]
enum SortCol { Time, RfqQuote, QuoteNos, NosEr, NosErFill, Duration }

#[component]
pub fn lifecycle_panel(
    messages: Signal<Vec<FixMessage>>,
    selected_idx: Signal<Option<usize>>,
    pro: bool,
) -> Element {
    // ── Signals ──
    let mut filter_sym:    Signal<String> = use_signal(String::new);
    let mut filter_status: Signal<String> = use_signal(|| "All".to_string());
    let selected_chain: Signal<Option<String>> = use_signal(|| None);
    let mut sort_col: Signal<SortCol> = use_signal(|| SortCol::Time);
    let mut sort_asc: Signal<bool>    = use_signal(|| true);
    let expanded_phase: Signal<Option<u8>>                        = use_signal(|| None);
    let drill_band:     Signal<Option<(SortCol, i64, i64)>>       = use_signal(|| None);

    // ── Memos ──
    let chains  = use_memo(move || build_lifecycle_chains(&messages.read()));

    // Per-phase latency extractions
    let rfq_quote_lats  = use_memo(move || chains.read().iter().filter_map(|c| c.rfq_to_quote_us).collect::<Vec<_>>());
    let quote_nos_lats  = use_memo(move || chains.read().iter().filter_map(|c| c.quote_to_nos_us).collect::<Vec<_>>());
    let nos_er_lats     = use_memo(move || chains.read().iter().filter_map(|c| c.nos_to_ack_us).collect::<Vec<_>>());
    let er_fill_lats    = use_memo(move || chains.read().iter().filter_map(|c| c.nos_to_fill_us).collect::<Vec<_>>());

    // Per-phase stats + charts
    let rfq_quote_stats = use_memo(move || compute_phase_stats(&rfq_quote_lats.read()));
    let quote_nos_stats = use_memo(move || compute_phase_stats(&quote_nos_lats.read()));
    let nos_er_stats    = use_memo(move || compute_phase_stats(&nos_er_lats.read()));
    let er_fill_stats   = use_memo(move || compute_phase_stats(&er_fill_lats.read()));
    // Draw the ECharts histogram whenever the expanded phase or latency data changes.
    use_effect(move || {
        let phase = *expanded_phase.read();
        let Some(idx) = phase else { return };
        let lats: Vec<i64> = match idx {
            0 => rfq_quote_lats.read().clone(),
            1 => quote_nos_lats.read().clone(),
            2 => nos_er_lats.read().clone(),
            3 => er_fill_lats.read().clone(),
            _ => return,
        };
        let js = latency_hist_js(&lats);
        spawn(async move { let _ = eval(&js).await; });
    });

    // Filtered + sorted chain list (capped at PAGE_SIZE for rendering)
    let filtered = use_memo(move || {
        let c     = chains.read();
        let sym_f = filter_sym.read().to_lowercase();
        let st_f  = filter_status.read().clone();
        let col   = *sort_col.read();
        let asc   = *sort_asc.read();
        let drill = *drill_band.read();
        let mut v: Vec<LifecycleChain> = c.iter().filter(|ch| {
            if !sym_f.is_empty() && !ch.symbol.to_lowercase().contains(sym_f.as_str()) { return false; }
            match st_f.as_str() {
                "Filled"    => matches!(ch.final_status, FinalStatus::Filled),
                "Partial"   => matches!(ch.final_status, FinalStatus::PartialFill),
                "Cancelled" => matches!(ch.final_status, FinalStatus::Cancelled),
                "Rejected"  => matches!(ch.final_status, FinalStatus::Rejected),
                "Open"      => matches!(ch.final_status, FinalStatus::Open),
                _           => true,
            }
        }).cloned().collect();
        let get_lat = |ch: &LifecycleChain, dc: SortCol| match dc {
            SortCol::RfqQuote  => ch.rfq_to_quote_us,
            SortCol::QuoteNos  => ch.quote_to_nos_us,
            SortCol::NosEr     => ch.nos_to_ack_us,
            SortCol::NosErFill => ch.nos_to_fill_us,
            _                  => None,
        };
        if let Some((dc, lo, hi)) = drill {
            // Drill-band: filter to the percentile window and sort worst-first
            v.retain(|ch| get_lat(ch, dc).map(|l| l >= lo && l <= hi).unwrap_or(false));
            v.sort_by(|a, b| get_lat(b, dc).unwrap_or(0).cmp(&get_lat(a, dc).unwrap_or(0)));
        } else {
            v.sort_by(|a, b| {
                let ord = match col {
                    SortCol::Time      => a.first_time_us.cmp(&b.first_time_us),
                    SortCol::RfqQuote  => cmp_opt(a.rfq_to_quote_us, b.rfq_to_quote_us),
                    SortCol::QuoteNos  => cmp_opt(a.quote_to_nos_us, b.quote_to_nos_us),
                    SortCol::NosEr     => cmp_opt(a.nos_to_ack_us,   b.nos_to_ack_us),
                    SortCol::NosErFill => cmp_opt(a.nos_to_fill_us,  b.nos_to_fill_us),
                    SortCol::Duration  => a.total_us.cmp(&b.total_us),
                };
                if asc { ord } else { ord.reverse() }
            });
        }
        v
    });

    // Inline timeline for selected chain
    let timeline_nodes: Memo<Vec<FlowNode>> = use_memo(move || {
        let sel = selected_chain.read();
        if let Some(ref id) = *sel {
            let c = chains.read();
            if let Some(ch) = c.iter().find(|c| &c.chain_id == id) {
                return build_chain_flow(&messages.read(), ch);
            }
        }
        Vec::new()
    });

    // ── Snapshot reads for RSX ──
    let chains_snap        = chains.read();
    let filtered_snap      = filtered.read();
    let timeline_snap      = timeline_nodes.read().clone();
    let sel_id             = selected_chain.read().clone();
    let filter_sym_val     = filter_sym.read().clone();
    let filter_st_val      = filter_status.read().clone();
    let sort_col_val       = *sort_col.read();
    let sort_asc_val       = *sort_asc.read();

    let rq_stats  = rfq_quote_stats.read().clone();
    let qn_stats  = quote_nos_stats.read().clone();
    let ne_stats  = nos_er_stats.read().clone();
    let ef_stats  = er_fill_stats.read().clone();
    let expanded_phase_val = *expanded_phase.read();
    let drill_band_val     = *drill_band.read();

    let total_chains   = chains_snap.len();
    let rfq_chains     = chains_snap.iter().filter(|c| c.has_rfq).count();
    let filtered_count = filtered_snap.len();
    let shown          = filtered_snap.len().min(PAGE_SIZE);

    let header_meta = format!(
        "{} chains · {} RFQ ({:.0}%) · {} NOS→ER · {} ER→Fill",
        total_chains, rfq_chains,
        rfq_chains as f64 / total_chains.max(1) as f64 * 100.0,
        rq_stats.as_ref().map(|s| s.count).unwrap_or(0),
        ef_stats.as_ref().map(|s| s.count).unwrap_or(0),
    );

    // ── Active phase detail (precomputed for RSX) ────────────────────────
    let active_stats: Option<PhaseStats> = match expanded_phase_val {
        Some(0) => rq_stats.clone(),
        Some(1) => qn_stats.clone(),
        Some(2) => ne_stats.clone(),
        Some(3) => ef_stats.clone(),
        _       => None,
    };
    let active_drill_col: SortCol = match expanded_phase_val {
        Some(1) => SortCol::QuoteNos,
        Some(2) => SortCol::NosEr,
        Some(3) => SortCol::NosErFill,
        _       => SortCol::RfqQuote,
    };
    let (dp50, dp95, dp99) = active_stats.as_ref()
        .map(|s| (s.p50_us, s.p95_us, s.p99_us))
        .unwrap_or((0, 0, 0));
    let adc = active_drill_col;
    let active_min_str  = active_stats.as_ref().map(|s| fmt_us(s.min_us)).unwrap_or_else(|| "—".into());
    let active_mean_str = active_stats.as_ref().map(|s| fmt_us(s.mean_us as i64)).unwrap_or_else(|| "—".into());
    let active_p50_str  = active_stats.as_ref().map(|s| fmt_us(s.p50_us)).unwrap_or_else(|| "—".into());
    let active_p95_str  = active_stats.as_ref().map(|s| fmt_us(s.p95_us)).unwrap_or_else(|| "—".into());
    let active_p99_str  = active_stats.as_ref().map(|s| fmt_us(s.p99_us)).unwrap_or_else(|| "—".into());
    let active_max_str  = active_stats.as_ref().map(|s| fmt_us(s.max_us)).unwrap_or_else(|| "—".into());
    let active_count    = active_stats.as_ref().map(|s| s.count).unwrap_or(0);
    let drill_desc: Option<String> = drill_band_val.map(|(col, lo, hi)| {
        let phase = match col {
            SortCol::RfqQuote  => "RFQ→Quote",
            SortCol::QuoteNos  => "Quote→NOS",
            SortCol::NosEr     => "NOS→ER",
            SortCol::NosErFill => "ER→Fill",
            _                  => "latency",
        };
        if hi >= i64::MAX / 2 {
            format!("Outliers ≥P99  (≥{})  ·  {}  ·  {} chains", fmt_us(lo), phase, filtered_count)
        } else {
            format!("{} – {}  ·  {}  ·  {} chains", fmt_us(lo), fmt_us(hi), phase, filtered_count)
        }
    });

    rsx! {
        div { class: "latency-panel",

            // ── Header ──────────────────────────────────────────────────────
            div { class: "latency-header",
                div { class: "latency-header-left",
                    h2 { class: "latency-title", "Trade Lifecycle Reconstructor" }
                    span { class: "latency-header-meta", "{header_meta}" }
                }
                if pro && total_chains > 0 {
                    button {
                        class: "btn-export-csv",
                        onclick: move |_| {
                            let chains_snap = filtered.read().clone();
                            spawn(async move {
                                let tag = now_tag();
                                if let Some(file) = rfd::AsyncFileDialog::new()
                                    .set_file_name(&format!("lifecycle_{tag}.csv"))
                                    .add_filter("CSV", &["csv"])
                                    .save_file()
                                    .await
                                {
                                    let csv = chains_to_csv(&chains_snap);
                                    let _ = std::fs::write(file.path(), csv.as_bytes());
                                }
                            });
                        },
                        "Export CSV"
                    }
                }
            }

            // ── Phase Overview: 4 cards + expandable detail ─────────────────
            div { class: "latency-section phase-overview-wrap phase-light",

                // Row of 4 clickable phase cards
                div { class: "phase-cards-row",
                    // RFQ → QUOTE
                    {
                        let active = expanded_phase_val == Some(0);
                        let cls    = if active { "phase-card phase-card-active" } else { "phase-card" };
                        let health = rq_stats.as_ref().map(|s| latency_health(s.p50_us)).unwrap_or("health-none");
                        let p50_s  = rq_stats.as_ref().map(|s| fmt_us(s.p50_us)).unwrap_or_else(|| "—".into());
                        let cnt_s  = rq_stats.as_ref().map(|s| format!("{}", s.count)).unwrap_or_else(|| "—".into());
                        rsx! {
                            div { class: "{cls}",
                                onclick: move |_| {
                                    let mut ep = expanded_phase;
                                    ep.set(if *ep.read() == Some(0) { None } else { Some(0) });
                                    let mut db = drill_band; db.set(None);
                                },
                                div { class: "phase-card-label", "RFQ → QUOTE" }
                                div { class: "phase-card-p50 {health}", "{p50_s}" }
                                div { class: "phase-card-sub", "P50  ·  {cnt_s} obs" }
                                span { class: "phase-card-caret", if active { "▲" } else { "▼" } }
                            }
                        }
                    }
                    // QUOTE → NOS
                    {
                        let active = expanded_phase_val == Some(1);
                        let cls    = if active { "phase-card phase-card-active" } else { "phase-card" };
                        let health = qn_stats.as_ref().map(|s| latency_health(s.p50_us)).unwrap_or("health-none");
                        let p50_s  = qn_stats.as_ref().map(|s| fmt_us(s.p50_us)).unwrap_or_else(|| "—".into());
                        let cnt_s  = qn_stats.as_ref().map(|s| format!("{}", s.count)).unwrap_or_else(|| "—".into());
                        rsx! {
                            div { class: "{cls}",
                                onclick: move |_| {
                                    let mut ep = expanded_phase;
                                    ep.set(if *ep.read() == Some(1) { None } else { Some(1) });
                                    let mut db = drill_band; db.set(None);
                                },
                                div { class: "phase-card-label", "QUOTE → NOS" }
                                div { class: "phase-card-p50 {health}", "{p50_s}" }
                                div { class: "phase-card-sub", "P50  ·  {cnt_s} obs" }
                                span { class: "phase-card-caret", if active { "▲" } else { "▼" } }
                            }
                        }
                    }
                    // NOS → ER
                    {
                        let active = expanded_phase_val == Some(2);
                        let cls    = if active { "phase-card phase-card-active" } else { "phase-card" };
                        let health = ne_stats.as_ref().map(|s| latency_health(s.p50_us)).unwrap_or("health-none");
                        let p50_s  = ne_stats.as_ref().map(|s| fmt_us(s.p50_us)).unwrap_or_else(|| "—".into());
                        let cnt_s  = ne_stats.as_ref().map(|s| format!("{}", s.count)).unwrap_or_else(|| "—".into());
                        rsx! {
                            div { class: "{cls}",
                                onclick: move |_| {
                                    let mut ep = expanded_phase;
                                    ep.set(if *ep.read() == Some(2) { None } else { Some(2) });
                                    let mut db = drill_band; db.set(None);
                                },
                                div { class: "phase-card-label", "NOS → ER" }
                                div { class: "phase-card-p50 {health}", "{p50_s}" }
                                div { class: "phase-card-sub", "P50  ·  {cnt_s} obs" }
                                span { class: "phase-card-caret", if active { "▲" } else { "▼" } }
                            }
                        }
                    }
                    // ER → FILL
                    {
                        let active = expanded_phase_val == Some(3);
                        let cls    = if active { "phase-card phase-card-active" } else { "phase-card" };
                        let health = ef_stats.as_ref().map(|s| latency_health(s.p50_us)).unwrap_or("health-none");
                        let p50_s  = ef_stats.as_ref().map(|s| fmt_us(s.p50_us)).unwrap_or_else(|| "—".into());
                        let cnt_s  = ef_stats.as_ref().map(|s| format!("{}", s.count)).unwrap_or_else(|| "—".into());
                        rsx! {
                            div { class: "{cls}",
                                onclick: move |_| {
                                    let mut ep = expanded_phase;
                                    ep.set(if *ep.read() == Some(3) { None } else { Some(3) });
                                    let mut db = drill_band; db.set(None);
                                },
                                div { class: "phase-card-label", "ER → FILL" }
                                div { class: "phase-card-p50 {health}", "{p50_s}" }
                                div { class: "phase-card-sub", "P50  ·  {cnt_s} obs" }
                                span { class: "phase-card-caret", if active { "▲" } else { "▼" } }
                            }
                        }
                    }
                }

                // Expanded detail panel
                if expanded_phase_val.is_some() && active_stats.is_some() {
                    div { class: "phase-detail",
                        div { class: "phase-detail-meta",
                            span { class: "phase-detail-count", "{active_count} observations" }
                            span { class: "phase-detail-hint", "● click P50 · P95 · P99 to filter the chain table" }
                        }
                        div { id: "latency-hist", class: "latency-hist-echarts" }
                        div { class: "phase-stats-table",
                            // Min (not clickable — it's a single extreme value)
                            div { class: "phase-stat-cell phase-stat-green",
                                div { class: "phase-stat-val", "{active_min_str}" }
                                div { class: "phase-stat-lbl", "Min" }
                            }
                            // Mean (not clickable)
                            div { class: "phase-stat-cell phase-stat-cyan",
                                div { class: "phase-stat-val", "{active_mean_str}" }
                                div { class: "phase-stat-lbl", "Mean" }
                            }
                            // P50 → shows orders in [P50, P95)
                            {
                                let is_active = drill_band_val.map(|(c,lo,_)| c == adc && lo == dp50).unwrap_or(false);
                                let cls = if is_active { "phase-stat-cell phase-stat-cyan phase-stat-drill phase-stat-drilling" }
                                          else { "phase-stat-cell phase-stat-cyan phase-stat-drill" };
                                rsx! {
                                    div { class: "{cls}", title: "Show typical orders (P50 – P95)",
                                        onclick: move |_| { let mut db = drill_band; db.set(Some((adc, dp50, dp95))); },
                                        div { class: "phase-stat-val", "{active_p50_str}" }
                                        div { class: "phase-stat-lbl", "P50  ●" }
                                    }
                                }
                            }
                            // P95 → shows orders in [P95, P99)
                            {
                                let is_active = drill_band_val.map(|(c,lo,_)| c == adc && lo == dp95).unwrap_or(false);
                                let cls = if is_active { "phase-stat-cell phase-stat-yellow phase-stat-drill phase-stat-drilling" }
                                          else { "phase-stat-cell phase-stat-yellow phase-stat-drill" };
                                rsx! {
                                    div { class: "{cls}", title: "Show slow orders (P95 – P99)",
                                        onclick: move |_| { let mut db = drill_band; db.set(Some((adc, dp95, dp99))); },
                                        div { class: "phase-stat-val", "{active_p95_str}" }
                                        div { class: "phase-stat-lbl", "P95  ●" }
                                    }
                                }
                            }
                            // P99 → shows outliers (≥ P99)
                            {
                                let is_active = drill_band_val.map(|(c,lo,_)| c == adc && lo == dp99).unwrap_or(false);
                                let cls = if is_active { "phase-stat-cell phase-stat-orange phase-stat-drill phase-stat-drilling" }
                                          else { "phase-stat-cell phase-stat-orange phase-stat-drill" };
                                rsx! {
                                    div { class: "{cls}", title: "Show outlier orders (≥ P99)",
                                        onclick: move |_| { let mut db = drill_band; db.set(Some((adc, dp99, i64::MAX))); },
                                        div { class: "phase-stat-val", "{active_p99_str}" }
                                        div { class: "phase-stat-lbl", "P99  ●" }
                                    }
                                }
                            }
                            // Max (not clickable)
                            div { class: "phase-stat-cell phase-stat-red",
                                div { class: "phase-stat-val", "{active_max_str}" }
                                div { class: "phase-stat-lbl", "Max" }
                            }
                        }
                    }
                }
            }

            // ── Lifecycle Reconstructor ──────────────────────────────────────
            div { class: "latency-section",
                div { class: "latency-section-title",
                    "LIFECYCLE RECONSTRUCTOR"
                    span { class: "latency-section-sub",
                        " — {filtered_count} chains · showing {shown} · click a row to view timeline"
                    }
                }

                // Drill-down banner (shown when a percentile cell is active)
                if let Some(ref desc) = drill_desc {
                    div { class: "drill-banner",
                        span { "{desc}" }
                        span {
                            class: "drill-banner-clear",
                            title: "Clear filter",
                            onclick: move |_| { let mut db = drill_band; db.set(None); },
                            "×"
                        }
                    }
                }

                // Filter bar
                div { class: "recon-filter-bar",
                    input {
                        class: "recon-filter-input",
                        r#type: "text",
                        placeholder: "Filter by symbol…",
                        value: "{filter_sym_val}",
                        oninput: move |e| filter_sym.set(e.value()),
                    }
                    {
                        const STATUSES: &[&str] = &["All", "Filled", "Partial", "Cancelled", "Rejected", "Open"];
                        STATUSES.iter().map(|&st| {
                            let active = filter_st_val == st;
                            let cls = if active { "recon-filter-btn recon-filter-btn-active" } else { "recon-filter-btn" };
                            let st_s = st.to_string();
                            rsx! {
                                button {
                                    class: "{cls}",
                                    onclick: move |_| filter_status.set(st_s.clone()),
                                    "{st}"
                                }
                            }
                        })
                    }
                }

                // Chain table
                if filtered_snap.is_empty() {
                    div { class: "latency-empty",
                        div { class: "latency-empty-icon", "🔍" }
                        p { class: "latency-empty-title", "No matching chains" }
                    }
                } else {
                    div { class: "table-wrap",
                        div { class: "tbl-header",
                            div { class: "tbl-chain-row",
                                span { "Chain ID" }
                                span { "Symbol" }
                                span { "Side" }
                                span { "Type" }
                                span { "Status" }
                                {
                                    let mk_hdr = |col: SortCol, label: &'static str| {
                                        let active = sort_col_val == col;
                                        let cls = if active { "lc-sort-hdr lc-sort-hdr-active" } else { "lc-sort-hdr" };
                                        let arrow = if active { if sort_asc_val { " ▲" } else { " ▼" } } else { "" };
                                        rsx! {
                                            span {
                                                class: "{cls}",
                                                onclick: move |_| {
                                                    if *sort_col.read() == col {
                                                        let cur = *sort_asc.read();
                                                        sort_asc.set(!cur);
                                                    } else {
                                                        sort_col.set(col);
                                                        sort_asc.set(true);
                                                    }
                                                },
                                                "{label}{arrow}"
                                            }
                                        }
                                    };
                                    rsx! {
                                        {mk_hdr(SortCol::RfqQuote,  "RFQ→Quote")}
                                        {mk_hdr(SortCol::QuoteNos,  "Quote→NOS")}
                                        {mk_hdr(SortCol::NosEr,     "NOS→ER")}
                                        {mk_hdr(SortCol::NosErFill, "ER→ER Filled")}
                                        {mk_hdr(SortCol::Duration,  "Duration")}
                                    }
                                }
                                span { "Msgs" }
                            }
                        }
                        div { class: "tbl-body latency-tbl-body",
                            {filtered_snap.iter().take(PAGE_SIZE).map(|ch| {
                                let is_sel   = sel_id.as_deref() == Some(ch.chain_id.as_str());
                                let row_cls  = if is_sel { "tbl-row tbl-chain-row flow-row-selected" } else { "tbl-row tbl-chain-row flow-row-clickable" };
                                let type_lbl = if ch.has_rfq { "RFQ" } else { "Direct" };
                                let type_cls = if ch.has_rfq { "chain-type-rfq" } else { "chain-type-direct" };
                                let st_lbl   = ch.final_status.label();
                                let st_cls   = ch.final_status.css_class();
                                let rfq_q    = ch.rfq_to_quote_us.map(fmt_us).unwrap_or_else(|| "—".into());
                                let qte_nos  = ch.quote_to_nos_us.map(fmt_us).unwrap_or_else(|| "—".into());
                                let nos_ack  = ch.nos_to_ack_us.map(fmt_us).unwrap_or_else(|| "—".into());
                                let nos_fill = ch.nos_to_fill_us.map(fmt_us).unwrap_or_else(|| "—".into());
                                let dur      = if ch.total_us > 0 { fmt_us(ch.total_us) } else { "—".into() };
                                let ack_cls  = match ch.nos_to_ack_us {
                                    Some(l) if l < 1_000   => "latency-cell-min",
                                    Some(l) if l < 10_000  => "latency-cell-mean",
                                    Some(l) if l < 100_000 => "latency-cell-p95",
                                    Some(_)                => "latency-cell-max",
                                    None                   => "lc-time",
                                };
                                let chain_id = ch.chain_id.clone();
                                let id_short = if ch.chain_id.len() > 14 { &ch.chain_id[..14] } else { ch.chain_id.as_str() };
                                let tl_lines = if is_sel { build_timeline_lines(&timeline_snap) } else { Vec::new() };
                                rsx! {
                                    div {
                                        class: "{row_cls}",
                                        onclick: move |_| {
                                            let mut sc = selected_chain;
                                            if sc.read().as_deref() == Some(chain_id.as_str()) {
                                                sc.set(None);
                                            } else {
                                                sc.set(Some(chain_id.clone()));
                                            }
                                        },
                                        span { class: "lc-clordid",    title: "{ch.chain_id}", "{id_short}" }
                                        span { class: "lc-symbol",     "{ch.symbol}" }
                                        span { class: "lc-side",       "{ch.side}" }
                                        span { class: "{type_cls}",    "{type_lbl}" }
                                        span { class: "{st_cls}",      "{st_lbl}" }
                                        span { class: "lc-time",       "{rfq_q}" }
                                        span { class: "lc-time",       "{qte_nos}" }
                                        span { class: "{ack_cls}",     "{nos_ack}" }
                                        span { class: "latency-cell-mean", "{nos_fill}" }
                                        span { class: "lc-time",       "{dur}" }
                                        span { class: "lc-count",      "{ch.msg_count}" }
                                    }
                                    if is_sel && !tl_lines.is_empty() {
                                        div { class: "chain-inline-expand",
                                            {tl_lines.into_iter().map(|line| {
                                                let indent_px = line.indent as u32 * 152;
                                                let indent_style = if indent_px > 0 {
                                                    format!("padding-left: {}px", indent_px)
                                                } else {
                                                    String::new()
                                                };
                                                rsx! {
                                                    div { class: "cit-line", style: "{indent_style}",
                                                        {line.segs.into_iter().map(|seg| {
                                                            let arrow = if !seg.first {
                                                                format!(" ──{}──► ", fmt_us_short(seg.delta_us))
                                                            } else {
                                                                String::new()
                                                            };
                                                            let cancel_pfx = if seg.is_cancel { "└─ " } else { "" };
                                                            let sub = if !seg.sublabel.is_empty() {
                                                                format!(" {}", seg.sublabel)
                                                            } else {
                                                                String::new()
                                                            };
                                                            let color = seg.color;
                                                            rsx! {
                                                                if !arrow.is_empty() {
                                                                    span { class: "cit-arrow", "{cancel_pfx}{arrow}" }
                                                                }
                                                                span {
                                                                    class: "cit-node",
                                                                    style: "color: {color}; border-color: {color}",
                                                                    "[{seg.label}{sub}]"
                                                                }
                                                            }
                                                        })}
                                                    }
                                                }
                                            })}
                                        }
                                    }
                                }
                            })}
                        }
                    }

                    if filtered_count > PAGE_SIZE {
                        div { class: "recon-more",
                            "Showing {shown} of {filtered_count} chains. Refine the symbol or status filter to narrow results."
                        }
                    }
                }
            }

            // ── Empty state ──────────────────────────────────────────────────
            if total_chains == 0 {
                div { class: "latency-empty",
                    div { class: "latency-empty-icon", "📊" }
                    p { class: "latency-empty-title", "No lifecycle data available" }
                    p { class: "latency-empty-hint", "Messages need:" }
                    ul { class: "latency-empty-list",
                        li { "ClOrdID (tag 11) on orders and execution reports" }
                        li { "QuoteReqID (tag 131) on RFQ flows" }
                        li { "Timestamps (tag 52 SendingTime)" }
                    }
                }
            }
        }
    }
}

// ── ECharts histogram builder ─────────────────────────────────────────────────

fn latency_hist_js(lats: &[i64]) -> String {
    const BUCKETS: &[(&str, i64, i64, &str)] = &[
        ("<10µs",      0,            10,       "#4ade80"),
        ("10–50µs",    10,           50,       "#86efac"),
        ("50–100µs",   50,           100,      "#a3e635"),
        ("0.1–0.5ms",  100,          500,      "#facc15"),
        ("0.5–1ms",    500,          1_000,    "#fb923c"),
        ("1–2ms",      1_000,        2_000,    "#f97316"),
        ("2–5ms",      2_000,        5_000,    "#ef4444"),
        ("5–10ms",     5_000,        10_000,   "#dc2626"),
        ("10–20ms",    10_000,       20_000,   "#b91c1c"),
        ("20–50ms",    20_000,       50_000,   "#991b1b"),
        ("50–100ms",   50_000,       100_000,  "#7f1d1d"),
        ("100–500ms",  100_000,      500_000,  "#6b21a8"),
        (">500ms",     500_000,      i64::MAX, "#4c1d95"),
    ];
    let mut counts = vec![0u64; BUCKETS.len()];
    for &l in lats {
        for (i, &(_, lo, hi, _)) in BUCKETS.iter().enumerate() {
            if l >= lo && l < hi { counts[i] += 1; break; }
        }
    }
    let labels: Vec<&str> = BUCKETS.iter().map(|b| b.0).collect();
    let colors: Vec<&str> = BUCKETS.iter().map(|b| b.3).collect();
    let labels_json = serde_json::to_string(&labels).unwrap_or_default();
    let counts_json = serde_json::to_string(&counts).unwrap_or_default();
    let colors_json = serde_json::to_string(&colors).unwrap_or_default();

    format!(r#"
(function init() {{
    if (typeof echarts === 'undefined') {{ setTimeout(init, 150); return; }}
    var el = document.getElementById('latency-hist');
    if (!el) {{ setTimeout(init, 150); return; }}
    var chart = echarts.getInstanceByDom(el) || echarts.init(el, null, {{renderer:'canvas'}});
    var labels = {labels};
    var counts = {counts};
    var colors = {colors};
    chart.setOption({{
        backgroundColor: 'transparent',
        tooltip: {{
            trigger: 'axis',
            axisPointer: {{ type: 'shadow' }},
            formatter: function(p) {{
                return '<b>' + p[0].name + '</b><br/>Count: ' + p[0].value;
            }}
        }},
        grid: {{ left: '4%', right: '2%', top: '10px', bottom: '72px', containLabel: true }},
        xAxis: {{
            type: 'category',
            data: labels,
            axisLabel: {{
                color: '#6b7280', fontSize: 10, rotate: 40, interval: 0
            }},
            axisLine: {{ lineStyle: {{ color: '#374151' }} }},
            axisTick: {{ show: false }}
        }},
        yAxis: {{
            type: 'value',
            axisLabel: {{ color: '#6b7280', fontSize: 10 }},
            splitLine: {{ lineStyle: {{ color: '#1f2937' }} }}
        }},
        series: [{{
            type: 'bar',
            data: counts.map(function(v, i) {{
                return {{ value: v, itemStyle: {{ color: colors[i], borderRadius: [3,3,0,0] }} }};
            }}),
            barMaxWidth: 36,
            label: {{
                show: true, position: 'top', color: '#9ca3af', fontSize: 9,
                formatter: function(p) {{ return p.value > 0 ? p.value : ''; }}
            }}
        }}]
    }}, true);
}})();
"#, labels = labels_json, counts = counts_json, colors = colors_json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_all;

    #[test]
    fn test_rfq_chain_latencies() {
        let input = "8=FIX.4.4|9=118|35=R|34=2|49=CITIFX|52=20240315-07:00:01.012000|56=FXECN|131=QR00000001|146=1|55=NZD/USD|38=11500000|54=1|64=20240319|10=020|\
8=FIX.4.4|9=160|35=S|34=2|49=FXECN|52=20240315-07:00:01.012404|56=CITIFX|117=QT00000001|131=QR00000001|55=NZD/USD|132=0.60242|133=0.60254|134=11500000|135=11500000|64=20240319|10=026|\
8=FIX.4.4|9=193|35=D|34=3|49=CITIFX|52=20240315-07:00:01.604695|56=FXECN|11=CO00000001|1=CITI-HF-01|55=NZD/USD|54=1|38=11500000|40=D|44=0.60254|117=QT00000001|59=4|64=20240319|60=20240315-07:00:01.604695|21=1|10=158|\
8=FIX.4.4|9=188|35=8|34=3|49=FXECN|52=20240315-07:00:01.623833|56=CITIFX|37=OR00000001|11=CO00000001|17=EX00000001|150=0|39=0|55=NZD/USD|54=1|38=11500000|14=0|151=11500000|6=0|60=20240315-07:00:01.623833|10=019|\
8=FIX.4.4|9=216|35=8|34=5|49=FXECN|52=20240315-07:00:01.649611|56=CITIFX|37=OR00000001|11=CO00000001|17=EX00000003|150=F|39=2|55=NZD/USD|54=1|38=11500000|14=11500000|151=0|31=0.60254|32=5750000|6=0.60254|60=20240315-07:00:01.649611|10=085|";
        let msgs = parse_all(input);
        eprintln!("Parsed {} messages", msgs.len());
        for (i, m) in msgs.iter().enumerate() {
            eprintln!("  [{}] type={} time={}", i, m.msg_type_raw, m.time);
        }
        let chains = build_lifecycle_chains(&msgs);
        eprintln!("Built {} chains", chains.len());
        for ch in &chains {
            eprintln!("  chain_id={} has_rfq={} rfq_to_quote={:?} quote_to_nos={:?} nos_to_ack={:?}",
                ch.chain_id, ch.has_rfq, ch.rfq_to_quote_us, ch.quote_to_nos_us, ch.nos_to_ack_us);
        }
        let rfq_chain = chains.iter().find(|c| c.has_rfq).expect("should have RFQ chain");
        assert!(rfq_chain.rfq_to_quote_us.is_some(), "rfq_to_quote_us should be Some");
        assert!(rfq_chain.quote_to_nos_us.is_some(), "quote_to_nos_us should be Some");
        assert!(rfq_chain.nos_to_ack_us.is_some(), "nos_to_ack_us should be Some");
    }
}