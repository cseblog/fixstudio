//! Fill Quality Scorecard — per-counterparty, per-symbol metrics.

use std::collections::HashMap;

use crate::model::FixMessage;
use crate::session_health::parse_time_us;

// ── Tag helper ────────────────────────────────────────────────────────────────

fn tag_val<'a>(msg: &'a FixMessage, tag: u16) -> &'a str {
    msg.fields
        .iter()
        .find(|f| f.tag == tag)
        .map(|f| f.value_in(&msg.arena))
        .unwrap_or("")
}

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
pub struct ScorecardRow {
    pub counterparty:       String,
    pub symbol:             Option<String>,
    pub fill_rate:          f64,
    pub slippage_bps:       f64,
    pub partial_fill_rate:  f64,
    pub reject_rate:        f64,
    pub avg_ack_ms:         f64,
    pub avg_fill_ms:        f64,
    pub cancel_success_rate: f64,
    pub order_count:        u64,
}

#[derive(Clone, PartialEq)]
pub struct SizeBucketRow {
    pub counterparty: String,
    pub symbol:       String,
    pub bucket:       &'static str,
    pub order_count:  u64,
}

#[derive(Clone, PartialEq)]
pub struct FillQualityScorecard {
    /// Top-level rows — one per counterparty (symbol = None = aggregate).
    pub rows: Vec<ScorecardRow>,
    /// Detail rows — one per (counterparty, symbol) pair.
    pub detail_rows: Vec<ScorecardRow>,
    /// Size-bucket rows — one per (counterparty, symbol, bucket) triple.
    pub size_rows: Vec<SizeBucketRow>,
}

// ── Per-order context collected during a single message pass ──────────────────

struct OrderContext {
    counterparty:  String,
    symbol:        String,
    order_qty:     f64,
    price:         f64,   // Limit price from NOS (tag 44); 0 = market
    side:          String,
    nos_time_us:   i64,
    /// Time of first ExecutionReport.
    first_er_us:   Option<i64>,
    /// Time and CumQty/AvgPx of the final fill ER.
    last_fill_us:  Option<i64>,
    cum_qty:       f64,
    avg_px:        f64,
    fill_count:    u32,  // number of partial fill ERs
    is_rejected:   bool,
    is_cancelled:  bool,
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn build_scorecard(messages: &[FixMessage]) -> FillQualityScorecard {
    let orders = collect_order_contexts(messages);
    let cancel_counts = count_cancel_requests_and_rejects(messages);

    let rows        = aggregate_rows(&orders, &cancel_counts, None);
    let detail_rows = aggregate_rows(&orders, &cancel_counts, Some(()));
    let size_rows   = collect_size_rows(&orders);
    FillQualityScorecard { rows, detail_rows, size_rows }
}

fn size_bucket(qty: f64) -> &'static str {
    if qty < 1_000_000.0      { "< 1M"    }
    else if qty < 5_000_000.0  { "1M–5M"   }
    else if qty < 10_000_000.0 { "5M–10M"  }
    else if qty < 50_000_000.0 { "10M–50M" }
    else                       { "> 50M"   }
}

fn collect_size_rows(orders: &[OrderContext]) -> Vec<SizeBucketRow> {
    let mut map: HashMap<(String, String, &'static str), u64> = HashMap::new();
    for order in orders {
        if order.counterparty.is_empty() || order.symbol.is_empty() { continue; }
        let bucket = size_bucket(order.order_qty);
        *map.entry((order.counterparty.clone(), order.symbol.clone(), bucket))
            .or_insert(0) += 1;
    }
    map.into_iter().map(|((cp, sym, bucket), count)| SizeBucketRow {
        counterparty: cp,
        symbol:       sym,
        bucket,
        order_count:  count,
    }).collect()
}

// ── Collection pass ───────────────────────────────────────────────────────────

fn collect_order_contexts(messages: &[FixMessage]) -> Vec<OrderContext> {
    let mut orders: HashMap<String, OrderContext> = HashMap::new();

    for msg in messages.iter() {
        let msg_type = tag_val(msg, 35);
        match msg_type {
            "D" => handle_new_order(msg, &mut orders),
            "8" => handle_execution_report(msg, &mut orders),
            _   => {}
        }
    }

    orders.into_values().collect()
}

fn handle_new_order(msg: &FixMessage, orders: &mut HashMap<String, OrderContext>) {
    let cl_ord_id = tag_val(msg, 11).to_string();
    if cl_ord_id.is_empty() { return; }
    let Some(nos_us) = parse_time_us(tag_val(msg, 52)) else { return };

    let order_qty: f64 = tag_val(msg, 38).parse().unwrap_or(0.0);
    let price: f64     = tag_val(msg, 44).parse().unwrap_or(0.0);
    // NOS is sent BY the client bank TO the ECN/broker.
    // The counterparty is therefore the SENDER (tag 49), not the target.
    let counterparty = if !msg.sender.is_empty() {
        msg.sender.to_string()
    } else {
        tag_val(msg, 49).to_string()
    };

    orders.insert(cl_ord_id, OrderContext {
        counterparty,
        symbol:       msg.symbol.to_string(),
        order_qty,
        price,
        side:         msg.side.to_string(),
        nos_time_us:  nos_us,
        first_er_us:  None,
        last_fill_us: None,
        cum_qty:      0.0,
        avg_px:       0.0,
        fill_count:   0,
        is_rejected:  false,
        is_cancelled: false,
    });
}

fn handle_execution_report(msg: &FixMessage, orders: &mut HashMap<String, OrderContext>) {
    let cl_ord_id = tag_val(msg, 11).to_string();
    if cl_ord_id.is_empty() { return; }
    let Some(ctx) = orders.get_mut(&cl_ord_id) else { return };
    let Some(er_us) = parse_time_us(tag_val(msg, 52)) else { return };

    if ctx.first_er_us.is_none() {
        ctx.first_er_us = Some(er_us);
    }

    let exec_type  = tag_val(msg, 150);
    let ord_status = tag_val(msg, 39);

    match ord_status {
        "8" => { ctx.is_rejected  = true; }
        "4" => { ctx.is_cancelled = true; }
        _   => {}
    }

    if exec_type == "F" || (ord_status == "2" && exec_type != "4" && exec_type != "8") {
        ctx.fill_count   += 1;
        ctx.last_fill_us  = Some(er_us);
        let cum_qty: f64  = tag_val(msg, 14).parse().unwrap_or(0.0);
        let avg_px_str    = tag_val(msg, 6);
        let avg_px: f64   = if !avg_px_str.is_empty() {
            avg_px_str.parse().unwrap_or(0.0)
        } else {
            tag_val(msg, 31).parse().unwrap_or(0.0)
        };
        if cum_qty > 0.0 { ctx.cum_qty = cum_qty; }
        if avg_px  > 0.0 { ctx.avg_px  = avg_px; }
    }
}

// ── Cancel tracking ───────────────────────────────────────────────────────────

/// Returns a map of counterparty → (cancel_request_count, cancel_reject_count).
fn count_cancel_requests_and_rejects(messages: &[FixMessage]) -> HashMap<String, (u64, u64)> {
    let mut counts: HashMap<String, (u64, u64)> = HashMap::new();
    for msg in messages.iter() {
        let msg_type = tag_val(msg, 35);
        // Cancel request (35=F): client → ECN, counterparty = sender (the client bank).
        // Cancel reject  (35=9): ECN → client, counterparty = target (the client bank).
        let counterparty = match msg_type {
            "F" => if !msg.sender.is_empty() { msg.sender.as_str() } else { tag_val(msg, 49) },
            "9" => if !msg.target.is_empty() { msg.target.as_str() } else { tag_val(msg, 56) },
            _   => continue,
        };
        if counterparty.is_empty() { continue; }
        let entry = counts.entry(counterparty.to_string()).or_insert((0, 0));
        match msg_type {
            "F" => entry.0 += 1,
            "9" => entry.1 += 1,
            _   => {}
        }
    }
    counts
}

// ── Aggregation ───────────────────────────────────────────────────────────────

/// If `by_symbol` is `Some`, emit one row per (counterparty, symbol) pair.
/// If `None`, emit one aggregate row per counterparty.
fn aggregate_rows(
    orders: &[OrderContext],
    cancel_counts: &HashMap<String, (u64, u64)>,
    by_symbol: Option<()>,
) -> Vec<ScorecardRow> {
    // Group order indices by key.
    let mut groups: HashMap<(String, Option<String>), Vec<usize>> = HashMap::new();
    for (i, order) in orders.iter().enumerate() {
        let symbol_key = by_symbol.map(|_| order.symbol.clone());
        groups
            .entry((order.counterparty.clone(), symbol_key))
            .or_default()
            .push(i);
    }

    let mut rows: Vec<ScorecardRow> = groups
        .into_iter()
        .filter_map(|((counterparty, symbol), indices)| {
            build_scorecard_row(orders, &indices, &counterparty, symbol, cancel_counts)
        })
        .collect();

    rows.sort_unstable_by(|a, b| b.order_count.cmp(&a.order_count));
    rows
}

fn build_scorecard_row(
    orders: &[OrderContext],
    indices: &[usize],
    counterparty: &str,
    symbol: Option<String>,
    cancel_counts: &HashMap<String, (u64, u64)>,
) -> Option<ScorecardRow> {
    if indices.is_empty() { return None; }

    let mut total_order_qty = 0.0_f64;
    let mut total_cum_qty   = 0.0_f64;
    let mut filled_count    = 0_u64;
    let mut rejected_count  = 0_u64;
    let mut partial_count   = 0_u64;
    let mut slippage_sum    = 0.0_f64;
    let mut slippage_count  = 0_u64;
    let mut ack_latencies   : Vec<f64> = Vec::new();
    let mut fill_latencies  : Vec<f64> = Vec::new();

    for &idx in indices {
        let order = &orders[idx];
        total_order_qty += order.order_qty;
        total_cum_qty   += order.cum_qty;

        if order.is_rejected  { rejected_count += 1; }
        if order.last_fill_us.is_some() {
            filled_count += 1;
            if order.fill_count > 1 { partial_count += 1; }
        }

        // Ack latency.
        if let Some(er_us) = order.first_er_us {
            let lat = (er_us - order.nos_time_us) as f64 / 1_000.0;
            if lat >= 0.0 { ack_latencies.push(lat); }
        }

        // Fill latency.
        if let Some(fill_us) = order.last_fill_us {
            let lat = (fill_us - order.nos_time_us) as f64 / 1_000.0;
            if lat >= 0.0 { fill_latencies.push(lat); }
        }

        // Slippage (limit orders only).
        if order.price > 0.0 && order.avg_px > 0.0 {
            let bps = if order.side.starts_with('B') || order.side == "1" {
                (order.avg_px - order.price) / order.price * 10_000.0
            } else {
                (order.price - order.avg_px) / order.price * 10_000.0
            };
            slippage_sum   += bps;
            slippage_count += 1;
        }
    }

    let order_count   = indices.len() as u64;
    let denom         = order_count.max(1) as f64;
    let fill_rate     = if total_order_qty > 0.0 {
        total_cum_qty / total_order_qty
    } else {
        filled_count as f64 / denom
    };
    let slippage_bps  = if slippage_count > 0 { slippage_sum / slippage_count as f64 } else { 0.0 };
    let avg_ack_ms    = vec_mean(&ack_latencies);
    let avg_fill_ms   = vec_mean(&fill_latencies);

    let (cancel_requests, cancel_rejects) =
        cancel_counts.get(counterparty).copied().unwrap_or((0, 0));
    let cancel_success_rate = if cancel_requests > 0 {
        let successes = cancel_requests.saturating_sub(cancel_rejects);
        successes as f64 / cancel_requests as f64
    } else {
        1.0
    };

    Some(ScorecardRow {
        counterparty:        counterparty.to_string(),
        symbol,
        fill_rate:           fill_rate.clamp(0.0, 1.0),
        slippage_bps,
        partial_fill_rate:   partial_count as f64 / denom,
        reject_rate:         rejected_count as f64 / denom,
        avg_ack_ms,
        avg_fill_ms,
        cancel_success_rate: cancel_success_rate.clamp(0.0, 1.0),
        order_count,
    })
}

fn vec_mean(values: &[f64]) -> f64 {
    if values.is_empty() { return 0.0; }
    values.iter().sum::<f64>() / values.len() as f64
}
