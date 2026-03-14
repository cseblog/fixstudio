use dioxus::prelude::*;
use std::collections::HashMap;

use crate::model::FixMessage;

// ─── Data structures ─────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
struct OrderLatency {
    cl_ord_id: String,
    symbol: String,
    side: String,
    first_time: String,
    ack_latency_us: Option<i64>,
    fill_latency_us: Option<i64>,
    msg_count: usize,
}

#[derive(Clone, PartialEq)]
struct LatencyStats {
    total_orders: usize,
    orders_with_ack: usize,
    mean_us: f64,
    min_us: i64,
    max_us: i64,
    p50_us: i64,
    p95_us: i64,
    p99_us: i64,
}

#[derive(Clone, PartialEq)]
struct SymbolStats {
    symbol: String,
    total: usize,
    with_ack: usize,
    mean_us: f64,
    p95_us: i64,
    min_us: i64,
    max_us: i64,
}

// ─── Time parsing ─────────────────────────────────────────────────────────────

/// Parse FIX timestamp in either raw form "YYYYMMDD-HH:MM:SS[.ffffff]"
/// or the stored display form "YYYY-MM-DD HH:MM:SS[.ffffff]" → microseconds since midnight.
fn parse_fix_time_us(s: &str) -> Option<i64> {
    // Stored format: "YYYY-MM-DD HH:MM:SS[.ffffff]" — split on space
    // Raw format:    "YYYYMMDD-HH:MM:SS[.ffffff]"    — split on '-' after 8-char date
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
        let mult: i64 = 10i64.pow((6 - flen) as u32);
        us += fval * mult;
    }
    Some(us)
}

fn fmt_us(us: i64) -> String {
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

// ─── Data computation ─────────────────────────────────────────────────────────

fn build_latency_data(messages: &[FixMessage]) -> Vec<OrderLatency> {
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    for (i, msg) in messages.iter().enumerate() {
        if msg.cl_ord_id.is_empty() { continue; }
        let key = msg.cl_ord_id.to_string();
        let entry = groups.entry(key.clone()).or_insert_with(|| {
            order.push(key.clone());
            Vec::new()
        });
        entry.push(i);
    }

    let mut result = Vec::with_capacity(order.len());
    for key in &order {
        let Some(indices) = groups.get(key) else { continue };
        if indices.is_empty() { continue; }

        let first = &messages[indices[0]];
        let first_us = parse_fix_time_us(&first.time);

        // ACK latency: time to first Execution Report (35=8) after the first message
        let ack_latency_us = indices.iter()
            .skip(1)
            .map(|&i| &messages[i])
            .find(|m| m.msg_type_raw == "8")
            .and_then(|m| parse_fix_time_us(&m.time))
            .zip(first_us)
            .map(|(er, ord)| er - ord)
            .filter(|&d| d > 0);

        // Fill latency: time to last Execution Report
        let fill_latency_us = if indices.len() > 1 {
            indices.iter()
                .rev()
                .map(|&i| &messages[i])
                .find(|m| m.msg_type_raw == "8")
                .and_then(|m| parse_fix_time_us(&m.time))
                .zip(first_us)
                .map(|(er, ord)| er - ord)
                .filter(|&d| d > 0)
        } else {
            None
        };

        result.push(OrderLatency {
            cl_ord_id: key.clone(),
            symbol: first.symbol.to_string(),
            side: first.side.to_string(),
            first_time: first.time.to_string(),
            ack_latency_us,
            fill_latency_us,
            msg_count: indices.len(),
        });
    }
    result
}

fn compute_stats(orders: &[OrderLatency]) -> Option<LatencyStats> {
    let mut lats: Vec<i64> = orders.iter().filter_map(|o| o.ack_latency_us).collect();
    if lats.is_empty() { return None; }
    lats.sort_unstable();
    let n = lats.len();
    let sum: i64 = lats.iter().sum();
    let pct = |p: f64| lats[((p / 100.0) * (n - 1) as f64).round() as usize];
    Some(LatencyStats {
        total_orders: orders.len(),
        orders_with_ack: n,
        mean_us: sum as f64 / n as f64,
        min_us: lats[0],
        max_us: lats[n - 1],
        p50_us: pct(50.0),
        p95_us: pct(95.0),
        p99_us: pct(99.0),
    })
}

fn compute_symbol_stats(orders: &[OrderLatency]) -> Vec<SymbolStats> {
    let mut map: HashMap<String, Vec<i64>> = HashMap::new();
    let mut totals: HashMap<String, usize> = HashMap::new();
    let mut sym_order: Vec<String> = Vec::new();

    for o in orders {
        let sym = if o.symbol.is_empty() { "—".to_string() } else { o.symbol.clone() };
        *totals.entry(sym.clone()).or_insert(0) += 1;
        let entry = map.entry(sym.clone()).or_insert_with(|| {
            sym_order.push(sym.clone());
            Vec::new()
        });
        if let Some(l) = o.ack_latency_us { entry.push(l); }
    }

    let mut result: Vec<SymbolStats> = sym_order.iter().filter_map(|sym| {
        let lats = map.get(sym)?;
        let mut s = lats.clone();
        s.sort_unstable();
        let n = s.len();
        if n == 0 { return None; }
        let sum: i64 = s.iter().sum();
        Some(SymbolStats {
            symbol: sym.clone(),
            total: *totals.get(sym).unwrap_or(&0),
            with_ack: n,
            mean_us: sum as f64 / n as f64,
            p95_us: s[((0.95 * (n - 1) as f64).round() as usize).min(n - 1)],
            min_us: s[0],
            max_us: s[n - 1],
        })
    }).collect();

    result.sort_by(|a, b| b.total.cmp(&a.total));
    result.truncate(12);
    result
}

// ─── SVG chart generators ─────────────────────────────────────────────────────

const CHART_FONT: &str = "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif";

fn render_histogram(orders: &[OrderLatency]) -> String {
    // Bucket: (label, lo_us, hi_us, colour)
    let buckets: &[(&str, i64, i64, &str)] = &[
        ("<0.1ms",  0,        100,       "#8be9fd"),
        ("0.1–1ms", 100,      1_000,     "#50fa7b"),
        ("1–5ms",   1_000,    5_000,     "#50fa7b"),
        ("5–10ms",  5_000,    10_000,    "#f1fa8c"),
        ("10–50ms", 10_000,   50_000,    "#ffb86c"),
        ("50–100ms",50_000,   100_000,   "#ff79c6"),
        (">100ms",  100_000,  i64::MAX,  "#ff5555"),
    ];

    let mut counts = vec![0usize; buckets.len()];
    for o in orders {
        if let Some(l) = o.ack_latency_us {
            for (i, (_, lo, hi, _)) in buckets.iter().enumerate() {
                if l >= *lo && l < *hi { counts[i] += 1; break; }
            }
        }
    }

    let max_c = counts.iter().copied().max().unwrap_or(1).max(1);
    const VW: f64 = 480.0;
    const VH: f64 = 150.0;
    const PL: f64 = 46.0; const PR: f64 = 10.0;
    const PT: f64 = 10.0; const PB: f64 = 34.0;
    let pw = VW - PL - PR;
    let ph = VH - PT - PB;
    let nb = buckets.len() as f64;
    let bw = pw / nb - 5.0;

    let mut s = format!(
        r##"<svg viewBox="0 0 {VW} {VH}" xmlns="http://www.w3.org/2000/svg" style="width:100%;display:block"><style>text{{font-family:{CHART_FONT};fill:#6272a4}}</style>"##
    );
    s += &format!(r##"<rect x="{PL}" y="{PT}" width="{pw}" height="{ph}" fill="#1e1f29" rx="4"/>"##);

    // Grid lines + Y labels
    for i in 1..=4 {
        let y = PT + ph * (1.0 - i as f64 / 4.0);
        let v = max_c as f64 * i as f64 / 4.0;
        let lbl = if v >= 1000.0 { format!("{:.0}k", v / 1000.0) } else { format!("{:.0}", v) };
        s += &format!(r##"<line x1="{PL}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}" stroke="#343746" stroke-width="1"/>"##, PL + pw);
        s += &format!(r##"<text x="{:.1}" y="{:.1}" font-size="9" text-anchor="end">{lbl}</text>"##, PL - 4.0, y + 3.5);
    }
    s += &format!(r##"<text x="{:.1}" y="{:.1}" font-size="9" text-anchor="end">0</text>"##, PL - 4.0, PT + ph + 3.5);

    // Bars
    for (i, ((label, _, _, color), &count)) in buckets.iter().zip(counts.iter()).enumerate() {
        let bh = (count as f64 / max_c as f64) * ph;
        let bx = PL + i as f64 * (pw / nb) + 2.5;
        let by = PT + ph - bh;

        if bh > 0.5 {
            s += &format!(r##"<rect x="{bx:.1}" y="{by:.1}" width="{bw:.1}" height="{bh:.1}" fill="{color}" rx="3"/>"##);
            if bh > 18.0 {
                s += &format!(r##"<text x="{:.1}" y="{:.1}" font-size="9" text-anchor="middle" fill="#282a36" font-weight="700">{count}</text>"##,
                    bx + bw / 2.0, by + 13.0);
            }
        } else {
            s += &format!(r##"<rect x="{bx:.1}" y="{:.1}" width="{bw:.1}" height="2" fill="{color}" rx="1" opacity="0.3"/>"##, PT + ph - 2.0);
        }
        s += &format!(r##"<text x="{:.1}" y="{:.1}" font-size="9" text-anchor="middle">{label}</text>"##,
            bx + bw / 2.0, PT + ph + 17.0);
    }

    // Axes
    s += &format!(r##"<line x1="{PL}" y1="{PT}" x2="{PL}" y2="{:.1}" stroke="#6272a4" stroke-width="1"/>"##, PT + ph);
    s += &format!(r##"<line x1="{PL}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#6272a4" stroke-width="1"/>"##, PT + ph, PL + pw, PT + ph);
    s += "</svg>";
    s
}

fn render_scatter(orders: &[OrderLatency]) -> String {
    let points: Vec<(usize, i64)> = orders.iter().enumerate()
        .filter_map(|(i, o)| o.ack_latency_us.map(|l| (i, l)))
        .collect();

    if points.is_empty() {
        return format!(r##"<svg viewBox="0 0 480 150" xmlns="http://www.w3.org/2000/svg" style="width:100%;display:block"><style>text{{font-family:{CHART_FONT}}}</style><text x="240" y="75" fill="#6272a4" font-size="12" text-anchor="middle">No ACK latency data — ensure messages have ClOrdID (tag 11) and timestamps</text></svg>"##);
    }

    // Subsample to max 500 points for rendering performance
    const MAX_PTS: usize = 500;
    let step = (points.len() / MAX_PTS).max(1);
    let sampled: Vec<(usize, i64)> = points.iter().step_by(step).cloned().collect();
    let n = sampled.len();

    // Y axis capped at P99 so extreme outliers don't compress the chart
    let mut all_lats: Vec<i64> = points.iter().map(|(_, l)| *l).collect();
    all_lats.sort_unstable();
    let al = all_lats.len();
    let p50 = all_lats[(0.50 * (al - 1) as f64) as usize];
    let p95 = all_lats[((0.95 * (al - 1) as f64) as usize).min(al - 1)];
    let p99 = all_lats[((0.99 * (al - 1) as f64) as usize).min(al - 1)];
    let y_max = p99.max(1);

    const VW: f64 = 480.0;
    const VH: f64 = 150.0;
    const PL: f64 = 50.0; const PR: f64 = 30.0;
    const PT: f64 = 12.0; const PB: f64 = 22.0;
    let pw = VW - PL - PR;
    let ph = VH - PT - PB;

    let mut s = format!(
        r##"<svg viewBox="0 0 {VW} {VH}" xmlns="http://www.w3.org/2000/svg" style="width:100%;display:block"><style>text{{font-family:{CHART_FONT};fill:#6272a4}}</style>"##
    );
    s += &format!(r##"<rect x="{PL}" y="{PT}" width="{pw}" height="{ph}" fill="#1e1f29" rx="4"/>"##);

    // Grid + Y labels
    for i in 0..=4 {
        let frac = i as f64 / 4.0;
        let y = PT + ph * (1.0 - frac);
        let lv = (y_max as f64 * frac) as i64;
        let lbl = fmt_us_short(lv);
        if i > 0 {
            s += &format!(r##"<line x1="{PL}" y1="{y:.1}" x2="{:.1}" y2="{y:.1}" stroke="#343746" stroke-width="1"/>"##, PL + pw);
        }
        s += &format!(r##"<text x="{:.1}" y="{:.1}" font-size="9" text-anchor="end">{lbl}</text>"##, PL - 4.0, y + 3.5);
    }

    // P50 reference line
    let p50c = (p50.min(y_max) as f64 / y_max as f64).min(1.0);
    let p50y = PT + ph * (1.0 - p50c);
    s += &format!(r##"<line x1="{PL}" y1="{p50y:.1}" x2="{:.1}" y2="{p50y:.1}" stroke="#50fa7b" stroke-width="1" stroke-dasharray="4,3" opacity="0.55"/>"##, PL + pw);
    s += &format!(r##"<text x="{:.1}" y="{:.1}" font-size="8" fill="#50fa7b" opacity="0.8">P50 {}</text>"##, PL + pw + 2.0, p50y + 3.5, fmt_us_short(p50));

    // P95 reference line
    let p95c = (p95.min(y_max) as f64 / y_max as f64).min(1.0);
    let p95y = PT + ph * (1.0 - p95c);
    s += &format!(r##"<line x1="{PL}" y1="{p95y:.1}" x2="{:.1}" y2="{p95y:.1}" stroke="#ffb86c" stroke-width="1" stroke-dasharray="4,3" opacity="0.55"/>"##, PL + pw);
    s += &format!(r##"<text x="{:.1}" y="{:.1}" font-size="8" fill="#ffb86c" opacity="0.8">P95 {}</text>"##, PL + pw + 2.0, p95y + 3.5, fmt_us_short(p95));

    // Scatter points
    for (idx, (_, lat)) in sampled.iter().enumerate() {
        let cx = PL + (idx as f64 / (n - 1).max(1) as f64) * pw;
        let clamped = (*lat).min(y_max);
        let cy = PT + ph * (1.0 - clamped as f64 / y_max as f64);
        let pct = clamped as f64 / y_max as f64;
        let color = if pct < 0.30 { "#50fa7b" }
            else if pct < 0.60 { "#8be9fd" }
            else if pct < 0.85 { "#f1fa8c" }
            else { "#ff5555" };
        // Outliers above P99 shown faded at top
        let (cy_r, extra) = if *lat > y_max { (PT + 3.0, r##" opacity="0.3""##) } else { (cy, "") };
        s += &format!(r##"<circle cx="{cx:.1}" cy="{cy_r:.1}" r="2" fill="{color}"{extra}/>"##);
    }

    // Axes
    s += &format!(r##"<line x1="{PL}" y1="{PT}" x2="{PL}" y2="{:.1}" stroke="#6272a4" stroke-width="1"/>"##, PT + ph);
    s += &format!(r##"<line x1="{PL}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#6272a4" stroke-width="1"/>"##, PT + ph, PL + pw, PT + ph);
    s += &format!(r##"<text x="{:.1}" y="{VH}" font-size="9" text-anchor="middle">← order sequence →</text>"##, PL + pw / 2.0);

    s += "</svg>";
    s
}

// ─── Order flow chart ─────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
struct FlowNode {
    label: String,       // e.g. "ER: Partial\n500@150.38"
    sublabel: String,    // quantity@price or exec detail
    time_str: String,    // HH:MM:SS.fff
    time_us: i64,
    delta_us: i64,       // micros since previous node (0 for first)
    kind: FlowKind,
}

#[derive(Clone, PartialEq)]
enum FlowKind {
    NewOrder,
    ExecNew,
    ExecPartial,
    ExecFilled,
    ExecCanceled,
    ExecRejected,
    CancelReq,
    Other,
}

impl FlowKind {
    fn color(&self) -> &'static str {
        match self {
            FlowKind::NewOrder     => "#bd93f9",
            FlowKind::ExecNew      => "#8be9fd",
            FlowKind::ExecPartial  => "#f1fa8c",
            FlowKind::ExecFilled   => "#50fa7b",
            FlowKind::ExecCanceled => "#ffb86c",
            FlowKind::ExecRejected => "#ff5555",
            FlowKind::CancelReq    => "#ff79c6",
            FlowKind::Other        => "#6272a4",
        }
    }
    fn border_color(&self) -> &'static str {
        match self {
            FlowKind::ExecFilled   => "#50fa7b",
            FlowKind::ExecRejected => "#ff5555",
            FlowKind::ExecCanceled => "#ffb86c",
            _                      => "#44475a",
        }
    }
}

fn tag_val<'a>(msg: &'a FixMessage, tag: u16) -> &'a str {
    msg.fields.iter().find(|f| f.tag == tag).map(|f| f.value.as_str()).unwrap_or("")
}

fn build_order_flow(messages: &[FixMessage], cl_ord_id: &str) -> Vec<FlowNode> {
    // Collect all messages for this ClOrdID, sorted by timestamp
    let mut msgs: Vec<&FixMessage> = messages.iter()
        .filter(|m| m.cl_ord_id.as_str() == cl_ord_id)
        .collect();
    if msgs.is_empty() { return vec![]; }

    // Sort by parsed time, preserving original order for ties
    msgs.sort_by_key(|m| parse_fix_time_us(&m.time).unwrap_or(i64::MAX));

    let mut nodes = Vec::with_capacity(msgs.len());
    let mut prev_us: Option<i64> = None;

    for msg in msgs {
        let t = parse_fix_time_us(&msg.time).unwrap_or(0);
        let delta = prev_us.map(|p| t - p).unwrap_or(0).max(0);
        prev_us = Some(t);

        // Time display: take HH:MM:SS.fff from stored "YYYY-MM-DD HH:MM:SS[.fff]"
        let time_str = msg.time.find(' ')
            .map(|i| &msg.time[i+1..])
            .unwrap_or(msg.time.as_str())
            .to_string();

        let (label, sublabel, kind) = match msg.msg_type_raw.as_str() {
            "D" => ("NewOrder".into(), {
                let sym = msg.symbol.as_str();
                let side = msg.side.as_str();
                let qty  = tag_val(msg, 38);
                let price = tag_val(msg, 44);
                if !sym.is_empty() { format!("{} {} {}@{}", side, sym, qty, price) }
                else { String::new() }
            }, FlowKind::NewOrder),

            "8" => {
                let ord_status = tag_val(msg, 39);
                let exec_type  = tag_val(msg, 150);
                let last_qty   = tag_val(msg, 32);
                let last_px    = tag_val(msg, 31);
                let cum_qty    = tag_val(msg, 14);
                let leaves_qty = tag_val(msg, 151);

                let sublbl = if !last_qty.is_empty() && !last_px.is_empty() {
                    format!("{}@{}", last_qty, last_px)
                } else if !cum_qty.is_empty() {
                    format!("cum {}", cum_qty)
                } else { String::new() };

                let _ = leaves_qty; // available if needed

                let (lbl, k) = match ord_status {
                    "0" => ("ER: New",      FlowKind::ExecNew),
                    "1" => ("ER: Partial",  FlowKind::ExecPartial),
                    "2" => ("ER: Filled",   FlowKind::ExecFilled),
                    "4" => ("ER: Canceled", FlowKind::ExecCanceled),
                    "8" => ("ER: Rejected", FlowKind::ExecRejected),
                    _   => {
                        // Fall back to ExecType
                        match exec_type {
                            "0" => ("ER: New",      FlowKind::ExecNew),
                            "1" => ("ER: Partial",  FlowKind::ExecPartial),
                            "2" => ("ER: Filled",   FlowKind::ExecFilled),
                            "4" => ("ER: Canceled", FlowKind::ExecCanceled),
                            "8" => ("ER: Rejected", FlowKind::ExecRejected),
                            _   => ("ExecReport",   FlowKind::Other),
                        }
                    }
                };
                (lbl.into(), sublbl, k)
            }

            "F" | "9" => {
                ("CancelReq".into(), String::new(), FlowKind::CancelReq)
            }

            t => (format!("35={}", t), String::new(), FlowKind::Other),
        };

        nodes.push(FlowNode { label, sublabel, time_str, time_us: t, delta_us: delta, kind });
    }
    nodes
}

// ─── Flow SVG renderer ────────────────────────────────────────────────────────

fn render_flow_svg(nodes: &[FlowNode]) -> String {
    if nodes.is_empty() { return String::new(); }

    const NODE_W: f64 = 110.0;
    const NODE_H: f64 = 54.0;
    const GAP:    f64 = 70.0;   // horizontal gap between nodes
    const ROW_H:  f64 = 90.0;   // vertical spacing between swimlanes
    const PAD_X:  f64 = 16.0;
    const PAD_Y:  f64 = 20.0;
    const ARROW_Y: f64 = PAD_Y + NODE_H / 2.0;

    // Assign rows: primary flow on row 0, cancel-branch on row 1
    // A cancel branch starts after a CancelReq node
    let mut row_idx = vec![0usize; nodes.len()];
    let mut in_cancel = false;
    for (i, n) in nodes.iter().enumerate() {
        if matches!(n.kind, FlowKind::CancelReq) { in_cancel = true; }
        if in_cancel { row_idx[i] = 1; }
    }

    let max_col = nodes.len();
    let total_w = PAD_X * 2.0 + max_col as f64 * (NODE_W + GAP) - GAP;
    let rows_used = if in_cancel { 2 } else { 1 };
    let total_h = PAD_Y * 2.0 + rows_used as f64 * ROW_H + NODE_H;

    let mut s = format!(
        r##"<svg id="flow-svg" viewBox="0 0 {total_w:.0} {total_h:.0}" xmlns="http://www.w3.org/2000/svg" style="width:100%;display:block;overflow:visible"><style>text{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;fill:#f8f8f2}}.flow-sub{{fill:#6272a4}}</style>"##
    );

    // Background
    s += &format!(r##"<rect width="{total_w:.0}" height="{total_h:.0}" fill="#1a1b26" rx="8"/>"##);

    // Draw arrows + latency labels first (so nodes render on top)
    let mut prev_col: Option<(usize, usize)> = None; // (col, row)
    for (i, node) in nodes.iter().enumerate() {
        let row = row_idx[i];
        let nx = PAD_X + i as f64 * (NODE_W + GAP);
        let ny = PAD_Y + row as f64 * ROW_H;
        let node_mid_x = nx + NODE_W / 2.0;
        let node_mid_y = ny + NODE_H / 2.0;

        if let Some((pi, pr)) = prev_col {
            let px = PAD_X + pi as f64 * (NODE_W + GAP);
            let prev_mid_x = px + NODE_W / 2.0;
            let prev_mid_y = PAD_Y + pr as f64 * ROW_H + NODE_H / 2.0;

            // Arrow from right edge of prev node to left edge of this node
            let ax1 = px + NODE_W;
            let ay1 = prev_mid_y;
            let ax2 = nx;
            let ay2 = node_mid_y;

            let delta_lbl = if node.delta_us > 0 {
                fmt_us(node.delta_us)
            } else { String::new() };

            let arrow_color = if node.delta_us > 100_000 { "#ff5555" }
                else if node.delta_us > 10_000 { "#ffb86c" }
                else if node.delta_us > 1_000  { "#f1fa8c" }
                else { "#50fa7b" };

            if pr == row {
                // Same row: straight horizontal arrow
                let mid_x = (ax1 + ax2) / 2.0;
                s += &format!(
                    r##"<line x1="{ax1:.1}" y1="{ay1:.1}" x2="{ax2:.1}" y2="{ay2:.1}" stroke="{arrow_color}" stroke-width="1.5" marker-end="url(#ah)"/>"##
                );
                if !delta_lbl.is_empty() {
                    s += &format!(
                        r##"<text x="{mid_x:.1}" y="{:.1}" font-size="9" text-anchor="middle" fill="{arrow_color}">{delta_lbl}</text>"##,
                        ay1 - 5.0
                    );
                }
            } else {
                // Different row: elbow arrow
                let elbow_x = prev_mid_x + (NODE_W / 2.0 + GAP / 2.0);
                s += &format!(
                    r##"<polyline points="{ax1:.1},{ay1:.1} {elbow_x:.1},{ay1:.1} {elbow_x:.1},{ay2:.1} {ax2:.1},{ay2:.1}" fill="none" stroke="{arrow_color}" stroke-width="1.5" marker-end="url(#ah)"/>"##
                );
                if !delta_lbl.is_empty() {
                    s += &format!(
                        r##"<text x="{:.1}" y="{:.1}" font-size="9" text-anchor="middle" fill="{arrow_color}">{delta_lbl}</text>"##,
                        (elbow_x + ax2) / 2.0, ay2 - 5.0
                    );
                }
            }
            let _ = (node_mid_x, node_mid_y, prev_mid_x); // suppress warnings
        }
        prev_col = Some((i, row));
    }

    // Arrow-head marker definition
    s += r##"<defs><marker id="ah" markerWidth="7" markerHeight="7" refX="6" refY="3.5" orient="auto"><polygon points="0 0, 7 3.5, 0 7" fill="#6272a4"/></marker></defs>"##;

    // Draw nodes
    for (i, node) in nodes.iter().enumerate() {
        let row = row_idx[i];
        let nx = PAD_X + i as f64 * (NODE_W + GAP);
        let ny = PAD_Y + row as f64 * ROW_H;
        let color = node.kind.color();
        let border = node.kind.border_color();

        // Node box
        s += &format!(
            r##"<rect x="{nx:.1}" y="{ny:.1}" width="{NODE_W}" height="{NODE_H}" fill="#282a36" stroke="{border}" stroke-width="1.5" rx="6"/>"##
        );
        // Colored top bar
        s += &format!(
            r##"<rect x="{nx:.1}" y="{ny:.1}" width="{NODE_W}" height="4" fill="{color}" rx="3"/>"##
        );

        // Label text (centered, possibly two lines)
        let lx = nx + NODE_W / 2.0;
        if node.sublabel.is_empty() {
            s += &format!(
                r##"<text x="{lx:.1}" y="{:.1}" font-size="11" font-weight="600" text-anchor="middle" fill="{color}">{}</text>"##,
                ny + 26.0, node.label
            );
        } else {
            s += &format!(
                r##"<text x="{lx:.1}" y="{:.1}" font-size="11" font-weight="600" text-anchor="middle" fill="{color}">{}</text>"##,
                ny + 21.0, node.label
            );
            s += &format!(
                r##"<text x="{lx:.1}" y="{:.1}" font-size="9" text-anchor="middle" class="flow-sub">{}</text>"##,
                ny + 33.0, node.sublabel
            );
        }
        // Time stamp at bottom of node
        s += &format!(
            r##"<text x="{lx:.1}" y="{:.1}" font-size="8" text-anchor="middle" class="flow-sub">{}</text>"##,
            ny + NODE_H - 7.0, node.time_str
        );
    }

    s += "</svg>";
    s
}

// ─── Component ────────────────────────────────────────────────────────────────

#[component]
pub fn lifecycle_panel(
    messages: Signal<Vec<FixMessage>>,
    selected_idx: Signal<Option<usize>>,
) -> Element {
    let mut selected_flow_id: Signal<Option<String>> = use_signal(|| None);

    let data    = use_memo(move || build_latency_data(&messages.read()));
    let stats   = use_memo(move || compute_stats(&data.read()));
    let syms    = use_memo(move || compute_symbol_stats(&data.read()));
    let hist    = use_memo(move || render_histogram(&data.read()));
    let scatter = use_memo(move || render_scatter(&data.read()));

    // Build flow SVG only when an order is selected
    let flow_svg = use_memo(move || {
        if let Some(id) = selected_flow_id.read().as_deref() {
            let nodes = build_order_flow(&messages.read(), id);
            render_flow_svg(&nodes)
        } else {
            String::new()
        }
    });

    // Install zoom/pan on flow chart whenever flow_svg changes
    use_effect(move || {
        let svg = flow_svg.read().clone();
        if !svg.is_empty() {
            use dioxus::document::eval;
            eval(r##"
                (function() {
                    var wrap = document.getElementById('flow-wrap');
                    if (!wrap) return;
                    var scale = 1, tx = 0, ty = 0, dragging = false, startX = 0, startY = 0;
                    function apply() {
                        wrap.style.transform = 'translate('+tx+'px,'+ty+'px) scale('+scale+')';
                        wrap.style.transformOrigin = '0 0';
                    }
                    wrap.onwheel = function(e) {
                        e.preventDefault();
                        var rect = wrap.parentElement.getBoundingClientRect();
                        var mx = e.clientX - rect.left, my = e.clientY - rect.top;
                        var factor = e.deltaY < 0 ? 1.12 : 0.89;
                        tx = mx - (mx - tx) * factor;
                        ty = my - (my - ty) * factor;
                        scale = Math.min(Math.max(scale * factor, 0.25), 4);
                        apply();
                    };
                    wrap.onmousedown = function(e) {
                        dragging = true; startX = e.clientX - tx; startY = e.clientY - ty;
                    };
                    window.addEventListener('mousemove', function(e) {
                        if (!dragging) return;
                        tx = e.clientX - startX; ty = e.clientY - startY; apply();
                    });
                    window.addEventListener('mouseup', function() { dragging = false; });
                })();
            "##);
        }
    });

    let orders    = data.read();
    let st        = stats.read();
    let sym_list  = syms.read();
    let hist_svg  = hist.read().clone();
    let scat_svg  = scatter.read().clone();
    let flow_svg_snap = flow_svg.read().clone();

    let stats_snap: Option<LatencyStats> = (*st).clone();

    let header_meta = match &stats_snap {
        Some(s) => format!(
            "{} orders  ·  {} with ACK  ·  {:.1}% coverage",
            s.total_orders,
            s.orders_with_ack,
            s.orders_with_ack as f64 / s.total_orders.max(1) as f64 * 100.0,
        ),
        None => format!("{} order groups — no timestamp data found", orders.len()),
    };
    let sym_count = sym_list.len();

    let mut top_slow: Vec<OrderLatency> = orders.iter()
        .filter(|o| o.ack_latency_us.is_some())
        .cloned()
        .collect();
    top_slow.sort_by(|a, b| b.ack_latency_us.cmp(&a.ack_latency_us));
    top_slow.truncate(20);
    let top_slow_len = top_slow.len();

    let selected_id_snap: Option<String> = (*selected_flow_id.read()).clone();

    rsx! {
        div { class: "latency-panel",

            // ── Header ──
            div { class: "latency-header",
                div { class: "latency-header-left",
                    h2 { class: "latency-title", "Trade Latency Analysis" }
                    span { class: "latency-header-meta", "{header_meta}" }
                }
            }

            if let Some(s) = &stats_snap {

                // ── Summary stat cards ──
                div { class: "latency-section",
                    div { class: "latency-section-title", "SUMMARY STATISTICS" }
                    div { class: "latency-stat-row",
                        div { class: "latency-stat-item latency-stat-green",
                            div { class: "latency-stat-val", "{fmt_us(s.min_us)}" }
                            div { class: "latency-stat-lbl", "Min" }
                        }
                        div { class: "latency-stat-item latency-stat-cyan",
                            div { class: "latency-stat-val", "{fmt_us(s.mean_us as i64)}" }
                            div { class: "latency-stat-lbl", "Mean" }
                        }
                        div { class: "latency-stat-item latency-stat-cyan",
                            div { class: "latency-stat-val", "{fmt_us(s.p50_us)}" }
                            div { class: "latency-stat-lbl", "P50" }
                        }
                        div { class: "latency-stat-item latency-stat-yellow",
                            div { class: "latency-stat-val", "{fmt_us(s.p95_us)}" }
                            div { class: "latency-stat-lbl", "P95" }
                        }
                        div { class: "latency-stat-item latency-stat-orange",
                            div { class: "latency-stat-val", "{fmt_us(s.p99_us)}" }
                            div { class: "latency-stat-lbl", "P99" }
                        }
                        div { class: "latency-stat-item latency-stat-red",
                            div { class: "latency-stat-val", "{fmt_us(s.max_us)}" }
                            div { class: "latency-stat-lbl", "Max" }
                        }
                    }
                }

                // ── Histogram ──
                div { class: "latency-section",
                    div { class: "latency-section-title", "ACK LATENCY DISTRIBUTION" }
                    div { class: "latency-chart-sub",
                        "Count of orders per latency bucket — NewOrder → first Execution Report"
                    }
                    div { class: "latency-chart-wrap",
                        dangerous_inner_html: "{hist_svg}",
                    }
                }

                // ── Scatter plot ──
                div { class: "latency-section",
                    div { class: "latency-section-title", "LATENCY SCATTER PLOT" }
                    div { class: "latency-chart-sub",
                        "Each point = one order · Y = ACK latency · capped at P99 · outliers faded"
                    }
                    div { class: "latency-chart-wrap",
                        dangerous_inner_html: "{scat_svg}",
                    }
                }

                // ── Per-symbol breakdown ──
                if !sym_list.is_empty() {
                    div { class: "latency-section",
                        div { class: "latency-section-title",
                            "PER-SYMBOL BREAKDOWN"
                            span { class: "latency-section-sub", " (top {sym_count} by order count)" }
                        }
                        div { class: "table-wrap",
                            div { class: "tbl-header",
                                div { class: "tbl-sym-row",
                                    span { "Symbol" }
                                    span { "Orders" }
                                    span { "ACK %" }
                                    span { "Mean" }
                                    span { "P95" }
                                    span { "Min" }
                                    span { "Max" }
                                }
                            }
                            div { class: "tbl-body latency-tbl-body",
                                {sym_list.iter().map(|sym| {
                                    let ack_pct = format!("{:.0}%", sym.with_ack as f64 / sym.total.max(1) as f64 * 100.0);
                                    rsx! {
                                        div { class: "tbl-row tbl-sym-row",
                                            span { class: "lc-symbol", "{sym.symbol}" }
                                            span { class: "lc-qty", "{sym.total}" }
                                            span { class: "lc-count", "{ack_pct}" }
                                            span { class: "latency-cell-mean", "{fmt_us(sym.mean_us as i64)}" }
                                            span { class: "latency-cell-p95", "{fmt_us(sym.p95_us)}" }
                                            span { class: "latency-cell-min", "{fmt_us(sym.min_us)}" }
                                            span { class: "latency-cell-max", "{fmt_us(sym.max_us)}" }
                                        }
                                    }
                                })}
                            }
                        }
                    }
                }

                // ── Top slowest orders ──
                if !top_slow.is_empty() {
                    div { class: "latency-section",
                        div { class: "latency-section-title",
                            "TOP {top_slow_len} SLOWEST ORDERS"
                            span { class: "latency-section-sub", " — click a row to view its lifecycle flow" }
                        }
                        div { class: "table-wrap",
                            div { class: "tbl-header",
                                div { class: "tbl-slow-row",
                                    span { "ClOrdID" }
                                    span { "Symbol" }
                                    span { "Side" }
                                    span { "ACK" }
                                    span { "Fill" }
                                    span { "Msgs" }
                                    span { "Time" }
                                }
                            }
                            div { class: "tbl-body latency-tbl-body",
                                {top_slow.iter().map(|o| {
                                    let ack = o.ack_latency_us.map(fmt_us).unwrap_or_else(|| "—".into());
                                    let fill = o.fill_latency_us.map(fmt_us).unwrap_or_else(|| "—".into());
                                    let ack_class = match o.ack_latency_us {
                                        Some(l) if l < 1_000   => "latency-cell-min",
                                        Some(l) if l < 10_000  => "latency-cell-mean",
                                        Some(l) if l < 100_000 => "latency-cell-p95",
                                        _                       => "latency-cell-max",
                                    };
                                    let is_sel = selected_id_snap.as_deref() == Some(o.cl_ord_id.as_str());
                                    let row_class = if is_sel { "tbl-row tbl-slow-row flow-row-selected" } else { "tbl-row tbl-slow-row flow-row-clickable" };
                                    let id = o.cl_ord_id.clone();
                                    rsx! {
                                        div {
                                            class: "{row_class}",
                                            onclick: move |_| {
                                                let mut sf = selected_flow_id;
                                                if sf.read().as_deref() == Some(id.as_str()) {
                                                    sf.set(None);
                                                } else {
                                                    sf.set(Some(id.clone()));
                                                }
                                            },
                                            span { class: "lc-clordid", "{o.cl_ord_id}" }
                                            span { class: "lc-symbol",  "{o.symbol}" }
                                            span { class: "lc-side",    "{o.side}" }
                                            span { class: "{ack_class}", "{ack}" }
                                            span { class: "latency-cell-mean", "{fill}" }
                                            span { class: "lc-count",  "{o.msg_count}" }
                                            span { class: "lc-time",   "{o.first_time}" }
                                        }
                                    }
                                })}
                            }
                        }
                    }
                }

                // ── Order lifecycle flow chart ──
                if !flow_svg_snap.is_empty() {
                    div { class: "latency-section",
                        div { class: "latency-section-title",
                            "ORDER LIFECYCLE FLOW"
                            span { class: "latency-section-sub", " — scroll to zoom · drag to pan" }
                        }
                        div { class: "latency-chart-sub",
                            "Arrow color: "
                            span { style: "color:#50fa7b", "green <1ms " }
                            span { style: "color:#f1fa8c", "yellow <10ms " }
                            span { style: "color:#ffb86c", "orange <100ms " }
                            span { style: "color:#ff5555", "red ≥100ms" }
                        }
                        div { class: "flow-chart-viewport",
                            div { id: "flow-wrap",
                                dangerous_inner_html: "{flow_svg_snap}",
                            }
                        }
                    }
                }

            } else {
                // ── No data state ──
                div { class: "latency-empty",
                    div { class: "latency-empty-icon", "📊" }
                    p { class: "latency-empty-title", "No latency data available" }
                    p { class: "latency-empty-hint", "Messages need:" }
                    ul { class: "latency-empty-list",
                        li { "ClOrdID (tag 11) to group orders" }
                        li { "Timestamps (tag 52 SendingTime or tag 60 TransactTime)" }
                        li { "At least one Execution Report (35=8) per order" }
                    }
                    if orders.is_empty() {
                        p { class: "latency-empty-hint", "Load a FIX log with order flow data." }
                    }
                }
            }
        }
    }
}
