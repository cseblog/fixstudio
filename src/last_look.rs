//! Liquidity-provider scorecard built from a Quote → NewOrderSingle → ER chain.
//!
//! "Last-look" is the FX-venue practice of holding a quote for a few ms
//! after the taker hits it, then rejecting if the price moved against
//! them. This module measures it per LP and surfaces the bad actors.
//!
//! For each ClOrdID:
//!   • Find the matching Quote (link via tag 117 QuoteID on the NOS).
//!   • Hold time = NOS.SendingTime − Quote.SendingTime.
//!   • Final outcome = last ER's ExecType/OrdStatus.
//!
//! Aggregated per LP (= target compID of the NOS):
//!   • order_count, fills, rejects, fill_rate, reject_rate
//:   • hold-time p50, p95, p99 (microseconds)
//!
//! Pure functions only. UI rendering lives in `components::overview`.

use ahash::AHashMap as HashMap;

use crate::model::FixMessage;
use crate::session_health::parse_time_us;

#[derive(Clone, Debug, PartialEq)]
pub struct LpRow {
    pub lp:               String,    // target compID (counterparty receiving the order)
    pub orders:           u64,
    pub fills:            u64,
    pub rejects:          u64,
    pub fill_rate:        f64,       // 0.0 .. 1.0
    pub reject_rate:      f64,
    pub hold_p50_us:      i64,
    pub hold_p95_us:      i64,
    pub hold_p99_us:      i64,
    pub hold_count:       u64,       // sample size for hold-time stats
    /// True if reject_rate ≥ 5% AND p95 hold ≥ 50 ms — heuristic flag for
    /// "last-look offender". Pure heuristic; user judges.
    pub flagged:          bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct LpScorecard {
    pub rows: Vec<LpRow>,
}

/// Per-message tag-value lookup (small struct; messages are < 32 fields).
fn tag_val<'a>(msg: &'a FixMessage, tag: u16) -> &'a str {
    msg.fields.iter().find(|f| f.tag == tag)
        .map(|f| f.value_in(&msg.arena))
        .unwrap_or("")
}

/// Build the scorecard. Single pass over messages.
pub fn build_lp_scorecard(msgs: &[FixMessage]) -> LpScorecard {
    // Quote bookkeeping: QuoteID → (target, sending_time_us). target is the
    // taker's side (the recipient of the original quote-request); we use
    // the Quote sender as the LP throughout this module.
    let mut quotes: HashMap<String, (String, i64)> = HashMap::default();
    // ClOrdID → (lp, hold_us, has_first_er_outcome). When a NOS lands we
    // stash the hold-time + LP; ER messages then mutate the outcome.
    struct Order {
        lp:        String,
        hold_us:   Option<i64>,
        finalised: bool,
        filled:    bool,
        rejected:  bool,
    }
    let mut orders: HashMap<String, Order> = HashMap::default();

    for m in msgs {
        let mt = m.msg_type_raw.as_str();
        match mt {
            // Quote (S) — store sender (=LP) + send-time keyed by QuoteID.
            "S" => {
                let q = tag_val(m, 117);
                if q.is_empty() { continue; }
                if m.sender.is_empty() { continue; }
                let Some(us) = parse_time_us(&m.time) else { continue };
                quotes.insert(q.to_string(), (m.sender.to_string(), us));
            }
            // NewOrderSingle (D) — LP = target; hold = NOS.time - Quote.time
            // when the NOS references a known QuoteID.
            "D" => {
                if m.cl_ord_id.is_empty() { continue; }
                let lp = m.target.to_string();
                if lp.is_empty() { continue; }
                let nos_us = parse_time_us(&m.time);
                let qid    = tag_val(m, 117);
                let hold_us = match (nos_us, quotes.get(qid)) {
                    (Some(nu), Some(&(_, qu))) if nu >= qu => Some(nu - qu),
                    _ => None,
                };
                orders.entry(m.cl_ord_id.to_string()).or_insert(Order {
                    lp,
                    hold_us,
                    finalised: false,
                    filled:    false,
                    rejected:  false,
                });
            }
            // ExecutionReport (8) — finalise the order outcome on the FIRST
            // terminal ER (fill, reject, cancelled).
            "8" => {
                if m.cl_ord_id.is_empty() { continue; }
                let Some(ord) = orders.get_mut(m.cl_ord_id.as_str()) else { continue };
                if ord.finalised { continue; }
                let exec_type  = tag_val(m, 150);
                let ord_status = tag_val(m, 39);
                // F=trade fill, 2=trade fill (status), 8=reject, 4=cancelled
                if exec_type == "F" || (exec_type == "2" && ord_status == "2") {
                    ord.filled = true;
                    ord.finalised = true;
                } else if exec_type == "8" || ord_status == "8" {
                    ord.rejected = true;
                    ord.finalised = true;
                }
            }
            _ => {}
        }
    }

    // Aggregate per LP.
    struct Agg {
        orders:   u64,
        fills:    u64,
        rejects:  u64,
        holds:    Vec<i64>,
    }
    let mut agg: HashMap<String, Agg> = HashMap::default();
    for (_, o) in orders {
        let e = agg.entry(o.lp.clone()).or_insert(Agg {
            orders: 0, fills: 0, rejects: 0, holds: Vec::new(),
        });
        e.orders += 1;
        if o.filled   { e.fills   += 1; }
        if o.rejected { e.rejects += 1; }
        if let Some(h) = o.hold_us { e.holds.push(h); }
    }

    let mut rows: Vec<LpRow> = agg.into_iter()
        .map(|(lp, a)| {
            let denom = a.orders.max(1) as f64;
            let fill_rate   = a.fills   as f64 / denom;
            let reject_rate = a.rejects as f64 / denom;
            let (p50, p95, p99) = percentiles(&a.holds);
            let flagged = reject_rate >= 0.05 && p95 >= 50_000;
            LpRow {
                lp,
                orders:      a.orders,
                fills:       a.fills,
                rejects:     a.rejects,
                fill_rate,
                reject_rate,
                hold_p50_us: p50,
                hold_p95_us: p95,
                hold_p99_us: p99,
                hold_count:  a.holds.len() as u64,
                flagged,
            }
        })
        .collect();
    // Worst-actor first by default — high reject_rate.
    rows.sort_by(|a, b| b.reject_rate.partial_cmp(&a.reject_rate)
        .unwrap_or(std::cmp::Ordering::Equal));
    LpScorecard { rows }
}

/// Per-(LP, symbol) fill rate for the heat grid. Returns the top
/// `max_symbols` symbols (by order count) and one row per LP.
pub struct SymbolGrid {
    pub symbols: Vec<String>,
    pub rows:    Vec<SymbolGridRow>,
}
pub struct SymbolGridRow {
    pub lp:    String,
    /// Same order as `symbols`. `None` = no orders for this LP × symbol.
    pub rates: Vec<Option<f64>>,
}

pub fn build_symbol_grid(msgs: &[FixMessage], max_symbols: usize) -> SymbolGrid {
    // Pass 1: per-symbol order count → pick top N.
    let mut sym_counts: HashMap<String, u64> = HashMap::default();
    for m in msgs {
        if m.msg_type_raw == "D" && !m.symbol.is_empty() {
            *sym_counts.entry(m.symbol.to_string()).or_insert(0) += 1;
        }
    }
    let mut sym_sorted: Vec<(String, u64)> = sym_counts.into_iter().collect();
    sym_sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sym_sorted.truncate(max_symbols);
    let symbols: Vec<String> = sym_sorted.into_iter().map(|(s, _)| s).collect();
    let sym_idx: HashMap<String, usize> = symbols.iter().enumerate()
        .map(|(i, s)| (s.clone(), i)).collect();

    // Pass 2: per ClOrdID, capture (lp, symbol, filled).
    struct Slot { orders: u64, fills: u64 }
    // key = (lp, symbol_idx)
    let mut cells: HashMap<(String, usize), Slot> = HashMap::default();
    let mut order_lp_sym: HashMap<String, (String, usize)> = HashMap::default();

    for m in msgs {
        let mt = m.msg_type_raw.as_str();
        if mt == "D" {
            if m.cl_ord_id.is_empty() || m.target.is_empty() || m.symbol.is_empty() { continue; }
            let Some(&si) = sym_idx.get(m.symbol.as_str()) else { continue };
            let key = (m.target.to_string(), si);
            cells.entry(key.clone()).or_insert(Slot { orders: 0, fills: 0 }).orders += 1;
            order_lp_sym.entry(m.cl_ord_id.to_string()).or_insert(key);
        } else if mt == "8" {
            if m.cl_ord_id.is_empty() { continue; }
            let Some(key) = order_lp_sym.get(m.cl_ord_id.as_str()) else { continue };
            let exec_type  = tag_val(m, 150);
            let ord_status = tag_val(m, 39);
            if exec_type == "F" || (exec_type == "2" && ord_status == "2") {
                if let Some(slot) = cells.get_mut(key) {
                    slot.fills += 1;
                }
                order_lp_sym.remove(m.cl_ord_id.as_str()); // ack final outcome once
            } else if exec_type == "8" || ord_status == "8" {
                order_lp_sym.remove(m.cl_ord_id.as_str()); // count reject = no fill
            }
        }
    }

    // Build rows.
    let mut lp_set: Vec<String> = cells.keys().map(|(lp, _)| lp.clone()).collect();
    lp_set.sort();
    lp_set.dedup();
    let mut rows: Vec<SymbolGridRow> = lp_set.into_iter().map(|lp| {
        let rates: Vec<Option<f64>> = (0..symbols.len()).map(|si| {
            cells.get(&(lp.clone(), si)).and_then(|slot| {
                if slot.orders == 0 { None }
                else { Some(slot.fills as f64 / slot.orders as f64) }
            })
        }).collect();
        SymbolGridRow { lp, rates }
    }).collect();
    // Worst-fill-rate LP first.
    rows.sort_by(|a, b| {
        let am = a.rates.iter().filter_map(|x| *x).fold(1.0, f64::min);
        let bm = b.rates.iter().filter_map(|x| *x).fold(1.0, f64::min);
        am.partial_cmp(&bm).unwrap_or(std::cmp::Ordering::Equal)
    });
    SymbolGrid { symbols, rows }
}

// ── Per-LP drill-down ───────────────────────────────────────────────────────
//
// Built on demand when the operator clicks a row in the scorecard. Walks
// the messages a second time and gathers everything needed to answer
// "why is this LP flagged?":
//
//   • reject reasons     — bucketed by tag 58 text, top N
//   • hold-time buckets  — fixed log-ish bins so a histogram bar chart can
//                          be drawn from the result without re-binning
//   • worst-hold sample  — the 10 slowest holds with their raw ClOrdID +
//                          symbol + outcome for direct copy-paste to LP

#[derive(Clone, Debug, PartialEq)]
pub struct LpDrillRejectReason {
    pub reason: String,
    pub count:  u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LpDrillHoldBucket {
    pub label: &'static str,
    pub count: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LpDrillWorstHold {
    pub cl_ord_id: String,
    pub symbol:    String,
    pub hold_us:   i64,
    pub time:      String,
    pub outcome:   &'static str,  // "FILL" / "REJECT" / "OPEN"
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct LpDrill {
    pub lp:             String,
    pub reject_reasons: Vec<LpDrillRejectReason>,
    pub hold_buckets:   Vec<LpDrillHoldBucket>,
    pub worst_holds:    Vec<LpDrillWorstHold>,
}

const HOLD_BUCKETS_US: &[(i64, &str)] = &[
    (1_000,        "<1ms"),
    (5_000,        "1-5ms"),
    (10_000,       "5-10ms"),
    (25_000,       "10-25ms"),
    (50_000,       "25-50ms"),
    (100_000,      "50-100ms"),
    (250_000,      "100-250ms"),
    (i64::MAX,     "250ms+"),
];

pub fn build_lp_drill(msgs: &[FixMessage], lp: &str) -> LpDrill {
    if lp.is_empty() {
        return LpDrill::default();
    }

    // Need the same quote → NOS hold linkage as the scorecard. Re-run it
    // narrowly for this LP only.
    let mut quotes: HashMap<String, i64> = HashMap::default();
    for m in msgs {
        if m.msg_type_raw.as_str() != "S" { continue; }
        if m.sender.as_str() != lp { continue; }
        let q = tag_val(m, 117);
        if q.is_empty() { continue; }
        if let Some(us) = parse_time_us(&m.time) {
            quotes.insert(q.to_string(), us);
        }
    }

    struct Order {
        cl:      String,
        sym:     String,
        nos_us:  i64,
        hold_us: Option<i64>,
        outcome: &'static str,
    }
    let mut orders: HashMap<String, Order> = HashMap::default();

    let mut reject_counts: HashMap<String, u64> = HashMap::default();

    for m in msgs {
        match m.msg_type_raw.as_str() {
            "D" => {
                if m.target.as_str() != lp { continue; }
                if m.cl_ord_id.is_empty() { continue; }
                let qid     = tag_val(m, 117);
                let nos_us  = parse_time_us(&m.time).unwrap_or(0);
                let hold_us = quotes.get(qid).map(|&qu| (nos_us - qu).max(0));
                orders.insert(m.cl_ord_id.to_string(), Order {
                    cl:      m.cl_ord_id.to_string(),
                    sym:     m.symbol.to_string(),
                    nos_us,
                    hold_us,
                    outcome: "OPEN",
                });
            }
            "8" => {
                if m.sender.as_str() != lp { continue; }
                if m.cl_ord_id.is_empty() { continue; }
                let exec_type  = tag_val(m, 150);
                let ord_status = tag_val(m, 39);
                let is_fill   = exec_type == "F" || (exec_type == "2" && ord_status == "2");
                let is_reject = exec_type == "8" || ord_status == "8";
                if is_reject {
                    // Tag 58 = Text (reject reason). Falls back to a
                    // synthetic "(no reason given)" so the bar chart still
                    // accounts for every reject.
                    let reason = tag_val(m, 58);
                    let key = if reason.is_empty() {
                        "(no reason given)".to_string()
                    } else {
                        normalize_reason(reason)
                    };
                    *reject_counts.entry(key).or_insert(0) += 1;
                }
                if let Some(o) = orders.get_mut(m.cl_ord_id.as_str()) {
                    if o.outcome == "OPEN" {
                        if is_fill   { o.outcome = "FILL"; }
                        if is_reject { o.outcome = "REJECT"; }
                    }
                }
            }
            _ => {}
        }
    }

    // Bucket hold times.
    let mut buckets: Vec<LpDrillHoldBucket> = HOLD_BUCKETS_US.iter()
        .map(|(_, label)| LpDrillHoldBucket { label, count: 0 })
        .collect();
    for o in orders.values() {
        if let Some(h) = o.hold_us {
            for (i, (cap, _)) in HOLD_BUCKETS_US.iter().enumerate() {
                if h < *cap { buckets[i].count += 1; break; }
            }
        }
    }

    // Worst-10 hold times (skip None hold).
    let mut all: Vec<&Order> = orders.values()
        .filter(|o| o.hold_us.is_some())
        .collect();
    all.sort_by(|a, b| b.hold_us.unwrap().cmp(&a.hold_us.unwrap()));
    let worst_holds: Vec<LpDrillWorstHold> = all.into_iter().take(10).map(|o| {
        LpDrillWorstHold {
            cl_ord_id: o.cl.clone(),
            symbol:    o.sym.clone(),
            hold_us:   o.hold_us.unwrap(),
            // Render NOS time from the stored microseconds — fine for sort,
            // operator just needs a stable label per row.
            time:      fmt_us_clock(o.nos_us),
            outcome:   o.outcome,
        }
    }).collect();

    // Top-N reject reasons.
    let mut reasons: Vec<LpDrillRejectReason> = reject_counts.into_iter()
        .map(|(reason, count)| LpDrillRejectReason { reason, count })
        .collect();
    reasons.sort_by(|a, b| b.count.cmp(&a.count));
    reasons.truncate(8);

    LpDrill {
        lp:             lp.to_string(),
        reject_reasons: reasons,
        hold_buckets:   buckets,
        worst_holds,
    }
}

/// Collapse case + trim so "Quote expired" and "QUOTE EXPIRED " bucket
/// together. Keeps it conservative — no token squashing.
fn normalize_reason(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.len() <= 64 { trimmed.to_string() }
    else { format!("{}…", &trimmed[..63]) }
}

fn fmt_us_clock(us: i64) -> String {
    // Render microseconds since epoch-of-trading-day as HH:MM:SS.mmm.
    // For replay logs the date is already in the surrounding context, so
    // the clock portion is what the operator scans for.
    let total_s = us / 1_000_000;
    let frac_us = us % 1_000_000;
    let h = (total_s / 3600) % 24;
    let m = (total_s % 3600) / 60;
    let s = total_s % 60;
    let ms = frac_us / 1_000;
    format!("{h:02}:{m:02}:{s:02}.{ms:03}")
}

fn percentiles(s: &[i64]) -> (i64, i64, i64) {
    if s.is_empty() { return (0, 0, 0); }
    let mut v = s.to_vec();
    v.sort_unstable();
    let idx = |p: f64| ((p * (v.len() - 1) as f64).round() as usize).min(v.len() - 1);
    (v[idx(0.50)], v[idx(0.95)], v[idx(0.99)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_all;

    fn p(s: &str) -> Vec<FixMessage> { parse_all(s) }

    fn quote_nos_er(qid: &str, lp_taker_pair: (&str, &str), cl: &str,
                    q_time: &str, nos_time: &str, er_time: &str,
                    er_type: &str, sym: &str) -> String {
        // lp_taker_pair = (LP/quote-sender = NOS-target, taker = NOS-sender)
        let (lp, taker) = lp_taker_pair;
        format!(
            concat!(
                "8=FIX.4.4|9=1|35=S|49={lp}|56={taker}|34=1|52={qt}|117={qid}|55={sym}|10=000|",
                "8=FIX.4.4|9=1|35=D|49={taker}|56={lp}|34=2|52={nt}|11={cl}|117={qid}|55={sym}|54=1|38=100|40=2|10=000|",
                "8=FIX.4.4|9=1|35=8|49={lp}|56={taker}|34=3|52={et}|11={cl}|150={ex}|39={st}|10=000|",
            ),
            lp = lp, taker = taker, qid = qid, cl = cl,
            qt = q_time, nt = nos_time, et = er_time,
            sym = sym, ex = er_type,
            st = if er_type == "F" { "2" } else if er_type == "8" { "8" } else { "0" },
        )
    }

    #[test]
    fn fill_and_reject_are_counted() {
        let mut raw = String::new();
        raw.push_str(&quote_nos_er("Q1", ("LP_A", "ME"), "C1",
            "20240101-09:00:00.000", "20240101-09:00:00.010", "20240101-09:00:00.020",
            "F", "EURUSD"));
        raw.push_str(&quote_nos_er("Q2", ("LP_A", "ME"), "C2",
            "20240101-09:00:01.000", "20240101-09:00:01.010", "20240101-09:00:01.020",
            "8", "EURUSD"));
        let sc = build_lp_scorecard(&p(&raw));
        assert_eq!(sc.rows.len(), 1);
        let r = &sc.rows[0];
        assert_eq!(r.lp, "LP_A");
        assert_eq!(r.orders, 2);
        assert_eq!(r.fills,  1);
        assert_eq!(r.rejects, 1);
        assert!((r.fill_rate   - 0.5).abs() < 1e-9);
        assert!((r.reject_rate - 0.5).abs() < 1e-9);
    }

    #[test]
    fn hold_time_measured_against_quote() {
        // Quote at 09:00:00.000, NOS at 09:00:00.025 → hold = 25 ms = 25_000 us.
        let raw = quote_nos_er("Q1", ("LP_A", "ME"), "C1",
            "20240101-09:00:00.000", "20240101-09:00:00.025", "20240101-09:00:00.030",
            "F", "EURUSD");
        let sc = build_lp_scorecard(&p(&raw));
        let r = &sc.rows[0];
        assert_eq!(r.hold_p50_us, 25_000);
        assert_eq!(r.hold_count, 1);
    }

    #[test]
    fn flag_triggers_on_high_reject_plus_slow_hold() {
        // 10 orders, 8 rejects (80% reject rate), all with 80 ms hold time.
        let mut raw = String::new();
        for i in 0..10u32 {
            let ms = 0 + (i / 10) * 1000;
            let exec = if i < 8 { "8" } else { "F" };
            raw.push_str(&quote_nos_er(
                &format!("Q{i}"), ("LP_BAD", "ME"), &format!("C{i}"),
                &format!("20240101-09:00:{:02}.000", ms/1000),
                &format!("20240101-09:00:{:02}.080", ms/1000),  // 80ms hold
                &format!("20240101-09:00:{:02}.090", ms/1000),
                exec, "EURUSD"
            ));
        }
        let sc = build_lp_scorecard(&p(&raw));
        assert!(sc.rows[0].flagged);
        assert!(sc.rows[0].reject_rate >= 0.05);
        assert!(sc.rows[0].hold_p95_us >= 50_000);
    }

    #[test]
    fn lps_sorted_worst_first() {
        let mut raw = String::new();
        // LP_GOOD: 100% fill.
        raw.push_str(&quote_nos_er("Q1", ("LP_GOOD", "ME"), "C1",
            "20240101-09:00:00.000", "20240101-09:00:00.005", "20240101-09:00:00.010",
            "F", "EURUSD"));
        // LP_BAD: 100% reject.
        raw.push_str(&quote_nos_er("Q2", ("LP_BAD", "ME"), "C2",
            "20240101-09:00:01.000", "20240101-09:00:01.005", "20240101-09:00:01.010",
            "8", "EURUSD"));
        let sc = build_lp_scorecard(&p(&raw));
        assert_eq!(sc.rows[0].lp, "LP_BAD");
        assert_eq!(sc.rows[1].lp, "LP_GOOD");
    }

    #[test]
    fn drill_buckets_hold_times() {
        // Three orders with 5ms, 30ms, 100ms holds — different buckets.
        let mut raw = String::new();
        raw.push_str(&quote_nos_er("Q1", ("LP_A", "ME"), "C1",
            "20240101-09:00:00.000", "20240101-09:00:00.005", "20240101-09:00:00.006",
            "F", "EURUSD"));
        raw.push_str(&quote_nos_er("Q2", ("LP_A", "ME"), "C2",
            "20240101-09:00:01.000", "20240101-09:00:01.030", "20240101-09:00:01.031",
            "F", "EURUSD"));
        raw.push_str(&quote_nos_er("Q3", ("LP_A", "ME"), "C3",
            "20240101-09:00:02.000", "20240101-09:00:02.100", "20240101-09:00:02.101",
            "8", "EURUSD"));
        let d = build_lp_drill(&p(&raw), "LP_A");
        // Sum across buckets should be 3.
        let total: u64 = d.hold_buckets.iter().map(|b| b.count).sum();
        assert_eq!(total, 3);
        // Worst hold first.
        assert_eq!(d.worst_holds[0].cl_ord_id, "C3");
        assert_eq!(d.worst_holds[0].outcome,   "REJECT");
    }

    #[test]
    fn drill_buckets_reject_reasons() {
        // Two rejects with same reason text → one bucket of 2.
        let raw = concat!(
            // Quote + NOS + ER(reject) — reason "Price moved"
            "8=FIX.4.4|9=1|35=S|49=LP_A|56=ME|34=1|52=20240101-09:00:00.000|117=Q1|55=EURUSD|10=000|",
            "8=FIX.4.4|9=1|35=D|49=ME|56=LP_A|34=2|52=20240101-09:00:00.001|11=C1|117=Q1|55=EURUSD|54=1|38=100|40=2|10=000|",
            "8=FIX.4.4|9=1|35=8|49=LP_A|56=ME|34=3|52=20240101-09:00:00.002|11=C1|150=8|39=8|58=Price moved|10=000|",
            "8=FIX.4.4|9=1|35=S|49=LP_A|56=ME|34=4|52=20240101-09:00:01.000|117=Q2|55=EURUSD|10=000|",
            "8=FIX.4.4|9=1|35=D|49=ME|56=LP_A|34=5|52=20240101-09:00:01.001|11=C2|117=Q2|55=EURUSD|54=1|38=100|40=2|10=000|",
            "8=FIX.4.4|9=1|35=8|49=LP_A|56=ME|34=6|52=20240101-09:00:01.002|11=C2|150=8|39=8|58=Price moved|10=000|",
        );
        let d = build_lp_drill(&p(raw), "LP_A");
        assert_eq!(d.reject_reasons.len(), 1);
        assert_eq!(d.reject_reasons[0].reason, "Price moved");
        assert_eq!(d.reject_reasons[0].count, 2);
    }

    #[test]
    fn drill_unknown_lp_is_empty() {
        let raw = quote_nos_er("Q1", ("LP_A", "ME"), "C1",
            "20240101-09:00:00.000", "20240101-09:00:00.001", "20240101-09:00:00.002",
            "F", "EURUSD");
        let d = build_lp_drill(&p(&raw), "DOES_NOT_EXIST");
        assert_eq!(d.lp, "DOES_NOT_EXIST");
        assert!(d.worst_holds.is_empty());
        assert!(d.reject_reasons.is_empty());
    }

    #[test]
    fn symbol_grid_orders_top_symbols() {
        let mut raw = String::new();
        // 3 orders EURUSD, 1 order GBPUSD.
        for i in 0..3 {
            raw.push_str(&quote_nos_er(
                &format!("Q{i}"), ("LP", "ME"), &format!("C{i}"),
                "20240101-09:00:00.000", "20240101-09:00:00.001", "20240101-09:00:00.002",
                "F", "EURUSD"));
        }
        raw.push_str(&quote_nos_er(
            "Q9", ("LP", "ME"), "C9",
            "20240101-09:00:00.000", "20240101-09:00:00.001", "20240101-09:00:00.002",
            "F", "GBPUSD"));
        let grid = build_symbol_grid(&p(&raw), 5);
        assert_eq!(grid.symbols[0], "EURUSD");
        assert_eq!(grid.symbols[1], "GBPUSD");
        let row = &grid.rows[0];
        assert_eq!(row.lp, "LP");
        assert!((row.rates[0].unwrap() - 1.0).abs() < 1e-9);
        assert!((row.rates[1].unwrap() - 1.0).abs() < 1e-9);
    }
}
