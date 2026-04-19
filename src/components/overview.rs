//! Overview Session Analysis — three-tab panel: Summary, Fill Quality, Health.

use dioxus::prelude::*;
use dioxus::document::eval;

use crate::export::{messages_to_csv, now_tag};
use crate::fill_quality::{build_scorecard, FillQualityScorecard, ScorecardRow};
use crate::model::FixMessage;
use crate::session_health::{
    HealthDetail, HealthIssue, HealthIssueKind, IssueSeverity, SessionHealthReport,
    HeartbeatGapDetail, SequenceGapDetail, ResendDetail, ReconnectDetail,
    RateBurstDetail, LateCancelDetail, RejectedCancelDetail, parse_time_us,
};
use crate::session_summary::{build_session_summary, SessionSummary};

// ── Tab enum ──────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum OverviewTab {
    Summary,
    FillQuality,
    Health,
}

// ── Fill Quality view toggle ───────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum FqView { Table, Charts }

// ── Sort state for Fill Quality table ─────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum SortCol {
    Counterparty,
    Orders,
    FillRate,
    Slippage,
    RejectRate,
    AckMs,
    FillMs,
    CancelRate,
}

// ── Component ─────────────────────────────────────────────────────────────────

/// Combined output of all overview computations — computed once per message set.
#[derive(Clone, PartialEq)]
struct OverviewData {
    summary:   SessionSummary,
    scorecard: FillQualityScorecard,
}

#[component]
pub fn overview_panel(messages: Signal<Vec<FixMessage>>) -> Element {
    let mut active_tab           = use_signal(|| OverviewTab::Summary);
    let sort_col                 = use_signal(|| SortCol::Orders);
    let sort_asc                 = use_signal(|| false);
    let mut drill_counterparty: Signal<Option<String>> = use_signal(|| None);
    let fq_view: Signal<FqView>                        = use_signal(|| FqView::Charts);
    let mut computed: Signal<Option<OverviewData>>     = use_signal(|| None);

    // Off-thread computation via rayon — never blocks the render thread.
    use_effect(move || {
        let msgs: Vec<FixMessage> = messages.read().clone();
        computed.set(None);
        let (tx, rx) = tokio::sync::oneshot::channel::<OverviewData>();
        rayon::spawn(move || {
            let data = OverviewData {
                summary:   build_session_summary(&msgs),
                scorecard: build_scorecard(&msgs),
            };
            let _ = tx.send(data);
        });
        spawn(async move {
            if let Ok(data) = rx.await {
                computed.set(Some(data));
            }
        });
    });

    // Draw / redraw ECharts whenever the tab or data changes.
    use_effect(move || {
        let tab  = active_tab.read().clone();
        let view = fq_view.read().clone();
        let maybe_js = {
            let data_ref = computed.read();
            match (&tab, &view) {
                (OverviewTab::Summary, _) =>
                    data_ref.as_ref().map(|d| build_summary_charts_js(&d.scorecard)),
                (OverviewTab::FillQuality, FqView::Charts) =>
                    data_ref.as_ref().map(|d| build_charts_js(&d.scorecard)),
                (OverviewTab::Health, _) =>
                    data_ref.as_ref().map(|d| build_health_charts_js(&d.summary.health)),
                _ => None,
            }
        };
        if let Some(js) = maybe_js {
            spawn(async move { let _ = eval(&js).await; });
        }
    });

    let tab_val   = active_tab.read().clone();
    let drill_val = drill_counterparty.read().clone();
    let data_opt  = computed.read().clone();

    rsx! {
        div { class: "overview-panel",

            // ── Header ───────────────────────────────────────────────────────
            div { class: "panel-header",
                div { class: "panel-title",
                    if let Some(ref d) = data_opt {
                        span { class: "parse-stats",
                            "{d.summary.sender} → {d.summary.target}  ·  \
                            {d.summary.start_time} – {d.summary.end_time}  ·  \
                            {d.summary.total_messages} messages"
                        }
                    }
                }
                button {
                    class: "btn-export-csv",
                    onclick: move |_| {
                        let snap: Vec<FixMessage> = messages.read().clone();
                        spawn(async move {
                            let tag = now_tag();
                            if let Some(file) = rfd::AsyncFileDialog::new()
                                .set_file_name(&format!("session_overview_{tag}.csv"))
                                .add_filter("CSV", &["csv"])
                                .save_file()
                                .await
                            {
                                let csv = messages_to_csv(&snap);
                                let _ = std::fs::write(file.path(), csv.as_bytes());
                            }
                        });
                    },
                    "Export CSV"
                }
            }

            // ── Tab bar ──────────────────────────────────────────────────────
            div { class: "overview-tab-bar",
                button {
                    class: if tab_val == OverviewTab::Summary
                        { "overview-tab overview-tab-active" } else { "overview-tab" },
                    onclick: move |_| active_tab.set(OverviewTab::Summary),
                    "Session Summary"
                }
                button {
                    class: if tab_val == OverviewTab::FillQuality
                        { "overview-tab overview-tab-active" } else { "overview-tab" },
                    onclick: move |_| {
                        active_tab.set(OverviewTab::FillQuality);
                        drill_counterparty.set(None);
                    },
                    "Fill Quality"
                }
                button {
                    class: if tab_val == OverviewTab::Health
                        { "overview-tab overview-tab-active" } else { "overview-tab" },
                    onclick: move |_| active_tab.set(OverviewTab::Health),
                    "Health"
                    if let Some(ref d) = data_opt {
                        if !d.summary.health.issues.is_empty() {
                            span { class: "tab-badge-warn",
                                "{d.summary.health.issues.len()}"
                            }
                        }
                    }
                }
            }

            // ── Tab content ──────────────────────────────────────────────────
            div { class: "overview-content",
                if let Some(data) = data_opt {
                    {match tab_val {
                        OverviewTab::Summary     => render_summary(&data.summary, &data.scorecard),
                        OverviewTab::FillQuality => render_fill_quality(
                            &data.scorecard, sort_col, sort_asc, drill_counterparty, &drill_val, fq_view,
                        ),
                        OverviewTab::Health      => render_health(&data.summary.health),
                    }}
                } else {
                    div { class: "overview-loading",
                        "Computing session report…"
                    }
                }
            }
        }
    }
}

// ── Summary tab ───────────────────────────────────────────────────────────────

fn render_summary(s: &SessionSummary, sc: &crate::fill_quality::FillQualityScorecard) -> Element {
    let stats       = &s.order_stats;
    let lats        = &s.latency_stats;
    let has_cp_data = !sc.rows.is_empty();

    rsx! {
        div { class: "summary-layout",

            // ── Left: stats table ─────────────────────────────────────────────
            div { class: "summary-body",

                div { class: "summary-section",
                    div { class: "summary-row",
                        span { class: "summary-label", "Session" }
                        span { class: "summary-value summary-session-label", "{s.session_label}" }
                    }
                    if s.session_count > 1 {
                        div { class: "summary-row",
                            span { class: "summary-label", "Pairs" }
                            span { class: "summary-value", "{s.session_count} session pairs" }
                        }
                    }
                    div { class: "summary-row",
                        span { class: "summary-label", "Duration" }
                        span { class: "summary-value",
                            "{s.start_time}  —  {s.end_time}"
                            if !s.duration_str.is_empty() {
                                span { class: "summary-duration", "  ({s.duration_str})" }
                            }
                        }
                    }
                    div { class: "summary-row",
                        span { class: "summary-label", "Messages" }
                        span { class: "summary-value", "{s.total_messages}" }
                    }
                }

                div { class: "summary-divider" }

                div { class: "summary-section",
                    div { class: "summary-row",
                        span { class: "summary-label", "Orders" }
                        span { class: "summary-value summary-bold", "{stats.total}" }
                    }
                    div { class: "summary-row summary-sub",
                        span { class: "summary-label", "  Filled" }
                        span { class: "summary-value",
                            "{stats.filled}"
                            span { class: "summary-pct summary-pct-green", "  ({stats.fill_pct:.1}%)" }
                        }
                    }
                    div { class: "summary-row summary-sub",
                        span { class: "summary-label", "  Cancelled" }
                        span { class: "summary-value",
                            "{stats.cancelled}"
                            span { class: "summary-pct", "  ({stats.cancel_pct:.1}%)" }
                        }
                    }
                    div { class: "summary-row summary-sub",
                        span { class: "summary-label", "  Rejected" }
                        span { class: "summary-value",
                            "{stats.rejected}"
                            span {
                                class: if stats.rejected > 0 { "summary-pct summary-pct-warn" } else { "summary-pct" },
                                "  ({stats.reject_pct:.1}%)"
                            }
                        }
                    }
                }

                div { class: "summary-divider" }

                div { class: "summary-section",
                    div { class: "summary-row",
                        span { class: "summary-label", "Avg ack latency" }
                        span { class: "summary-value summary-mono", "{lats.avg_ack_ms:.2}ms" }
                    }
                    div { class: "summary-row",
                        span { class: "summary-label", "Avg fill latency" }
                        span { class: "summary-value summary-mono", "{lats.avg_fill_ms:.1}ms" }
                    }
                    if lats.worst_spike_ms > 0.0 {
                        div { class: "summary-row",
                            span { class: "summary-label", "Worst spike" }
                            span { class: "summary-value summary-mono summary-warn",
                                "{lats.worst_spike_ms:.0}ms"
                                if let Some(ref t) = lats.worst_spike_time {
                                    span { class: "summary-spike-meta",
                                        "  at {t}"
                                        if lats.worst_spike_count > 0 {
                                            "  ({lats.worst_spike_count} orders)"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if !s.top_symbols.is_empty() {
                    div { class: "summary-divider" }
                    div { class: "summary-section",
                        div { class: "summary-row",
                            span { class: "summary-label", "Top symbols" }
                            span { class: "summary-value",
                                for (sym, count) in s.top_symbols.iter().take(5) {
                                    span { class: "summary-symbol",
                                        "{sym} "
                                        span { class: "summary-symbol-count", "({count})  " }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── Right: per-counterparty pie charts ────────────────────────────
            if has_cp_data {
                div { class: "summary-charts",
                    div { class: "summary-chart-block",
                        p { class: "summary-chart-label", "Fills by Counterparty" }
                        div { id: "summary-fill-pie", class: "summary-pie" }
                    }
                    div { class: "summary-chart-block",
                        p { class: "summary-chart-label", "Rejects by Counterparty" }
                        div { id: "summary-reject-pie", class: "summary-pie" }
                    }
                }
            }
        }
    }
}

// ── Fill Quality tab ──────────────────────────────────────────────────────────

fn render_fill_quality(
    scorecard:              &FillQualityScorecard,
    sort_col:               Signal<SortCol>,
    sort_asc:               Signal<bool>,
    mut drill_counterparty: Signal<Option<String>>,
    drill:                  &Option<String>,
    mut fq_view:            Signal<FqView>,
) -> Element {
    let view_val     = fq_view.read().clone();
    let sort_col_val = sort_col.read().clone();
    let sort_asc_val = *sort_asc.read();

    let mut rows: Vec<ScorecardRow> = if view_val == FqView::Table {
        let base: Vec<ScorecardRow> = if let Some(ref cp) = drill {
            scorecard.detail_rows.iter().filter(|r| &r.counterparty == cp).cloned().collect()
        } else {
            scorecard.rows.clone()
        };
        base
    } else {
        Vec::new()
    };
    if view_val == FqView::Table {
        sort_rows(&mut rows, &sort_col_val, sort_asc_val);
    }

    rsx! {
        div { class: "scorecard-wrap",

            // ── View toggle ──────────────────────────────────────────────────
            div { class: "fq-view-toggle",
                button {
                    class: if view_val == FqView::Table
                        { "fq-view-btn fq-view-btn-active" } else { "fq-view-btn" },
                    onclick: move |_| fq_view.set(FqView::Table),
                    "Table"
                }
                button {
                    class: if view_val == FqView::Charts
                        { "fq-view-btn fq-view-btn-active" } else { "fq-view-btn" },
                    onclick: move |_| {
                        drill_counterparty.set(None);
                        fq_view.set(FqView::Charts);
                    },
                    "Charts"
                }
            }

            // ── Charts view ──────────────────────────────────────────────────
            if view_val == FqView::Charts {
                div { class: "fq-charts-wrap",
                    div { class: "fq-chart-section",
                        p { class: "fq-chart-label",
                            "Fill Rate & Reject Rate by Counterparty"
                        }
                        div { id: "fq-bar-chart", class: "fq-chart" }
                    }
                    div { class: "fq-chart-section",
                        p { class: "fq-chart-label",
                            "Order Volume · Counterparty → Symbol  (click to drill down)"
                        }
                        div { id: "fq-tree-chart", class: "fq-treemap" }
                    }
                }

            // ── Table view ───────────────────────────────────────────────────
            } else {
                if let Some(ref cp) = drill {
                    div { class: "scorecard-breadcrumb",
                        button {
                            class: "scorecard-back-btn",
                            onclick: move |_| drill_counterparty.set(None),
                            "← All counterparties"
                        }
                        span { class: "scorecard-breadcrumb-sep", " / " }
                        span { "{cp} — by symbol" }
                    }
                }

                if rows.is_empty() {
                    div { class: "empty-state", "No order data available." }
                } else {
                    div { class: "scorecard-table-wrap",
                        div { class: "scorecard-table",
                            div { class: "scorecard-row scorecard-header",
                                {sc_sort_th("Counterparty", SortCol::Counterparty,
                                    &sort_col_val, sort_asc_val, sort_col, sort_asc)}
                                if drill.is_some() {
                                    span { class: "sc-cell sc-header-cell", "Symbol" }
                                }
                                {sc_sort_th("Orders",   SortCol::Orders,    &sort_col_val, sort_asc_val, sort_col, sort_asc)}
                                {sc_sort_th("Fill %",   SortCol::FillRate,  &sort_col_val, sort_asc_val, sort_col, sort_asc)}
                                {sc_sort_th("Slip bps", SortCol::Slippage,  &sort_col_val, sort_asc_val, sort_col, sort_asc)}
                                {sc_sort_th("Rej %",    SortCol::RejectRate,&sort_col_val, sort_asc_val, sort_col, sort_asc)}
                                {sc_sort_th("Ack ms",   SortCol::AckMs,     &sort_col_val, sort_asc_val, sort_col, sort_asc)}
                                {sc_sort_th("Fill ms",  SortCol::FillMs,    &sort_col_val, sort_asc_val, sort_col, sort_asc)}
                                {sc_sort_th("Cancel %", SortCol::CancelRate,&sort_col_val, sort_asc_val, sort_col, sort_asc)}
                            }
                            for row in rows.iter() {
                                {sc_data_row(row, drill, drill_counterparty)}
                            }
                        }
                    }
                }
            }
        }
    }
}

fn sc_sort_th(
    label:        &'static str,
    col:          SortCol,
    current:      &SortCol,
    asc:          bool,
    mut sort_col: Signal<SortCol>,
    mut sort_asc: Signal<bool>,
) -> Element {
    let active = current == &col;
    let arrow  = if active { if asc { " ▲" } else { " ▼" } } else { "" };
    let class  = if active {
        "sc-cell sc-header-cell sc-sorted"
    } else {
        "sc-cell sc-header-cell"
    };
    let label_with_arrow = format!("{label}{arrow}");
    rsx! {
        span {
            class: "{class}",
            onclick: move |_| {
                if *sort_col.read() == col {
                    let new_asc = !*sort_asc.read();
                    sort_asc.set(new_asc);
                } else {
                    sort_col.set(col.clone());
                    sort_asc.set(false);
                }
            },
            "{label_with_arrow}"
        }
    }
}

fn sc_data_row(
    row:                    &ScorecardRow,
    drill:                  &Option<String>,
    mut drill_counterparty: Signal<Option<String>>,
) -> Element {
    let fill_class = if row.fill_rate > 0.95 { "sc-cell sc-good" }
        else if row.fill_rate > 0.8           { "sc-cell sc-ok"   }
        else                                  { "sc-cell sc-bad"  };
    let rej_class  = if row.reject_rate > 0.05 { "sc-cell sc-bad" } else { "sc-cell" };
    let cp         = row.counterparty.clone();
    let is_drill   = drill.is_some();
    let sym_str    = row.symbol.clone().unwrap_or_default();
    let fill_pct   = format!("{:.1}%", row.fill_rate * 100.0);
    let rej_pct    = format!("{:.1}%", row.reject_rate * 100.0);
    let cancel_pct = format!("{:.0}%", row.cancel_success_rate * 100.0);

    rsx! {
        div {
            class: if is_drill { "scorecard-row" } else { "scorecard-row scorecard-row-clickable" },
            onclick: move |_| {
                if !is_drill {
                    drill_counterparty.set(Some(cp.clone()));
                }
            },
            span { class: "sc-cell sc-cp", "{row.counterparty}" }
            if is_drill {
                span { class: "sc-cell sc-sym", "{sym_str}" }
            }
            span { class: "sc-cell sc-num",  "{row.order_count}" }
            span { class: "{fill_class}",    "{fill_pct}" }
            span { class: "sc-cell sc-num",  "{row.slippage_bps:.2}" }
            span { class: "{rej_class}",     "{rej_pct}" }
            span { class: "sc-cell sc-num",  "{row.avg_ack_ms:.2}" }
            span { class: "sc-cell sc-num",  "{row.avg_fill_ms:.1}" }
            span { class: "sc-cell sc-num",  "{cancel_pct}" }
        }
    }
}

fn sort_rows(rows: &mut Vec<ScorecardRow>, col: &SortCol, asc: bool) {
    rows.sort_unstable_by(|a, b| {
        let ordering = match col {
            SortCol::Counterparty => a.counterparty.cmp(&b.counterparty),
            SortCol::Orders       => a.order_count.cmp(&b.order_count),
            SortCol::FillRate     => a.fill_rate.partial_cmp(&b.fill_rate)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortCol::Slippage     => a.slippage_bps.partial_cmp(&b.slippage_bps)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortCol::RejectRate   => a.reject_rate.partial_cmp(&b.reject_rate)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortCol::AckMs        => a.avg_ack_ms.partial_cmp(&b.avg_ack_ms)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortCol::FillMs       => a.avg_fill_ms.partial_cmp(&b.avg_fill_ms)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortCol::CancelRate   => a.cancel_success_rate
                .partial_cmp(&b.cancel_success_rate)
                .unwrap_or(std::cmp::Ordering::Equal),
        };
        if asc { ordering } else { ordering.reverse() }
    });
}

// ── Health tab ────────────────────────────────────────────────────────────────

fn render_health(report: &SessionHealthReport) -> Element {
    if report.issues.is_empty() {
        return rsx! {
            div { class: "health-empty",
                span { class: "health-ok-icon", "✓" }
                span { "No issues detected. Session looks healthy." }
            }
        };
    }

    // Pre-compute all display data before entering RSX.
    struct Card {
        idx:             usize,
        kind_label:      &'static str,
        sev_class:       &'static str,
        sev_icon:        &'static str,
        technical_desc:  String,
        business_impact: String,
        detail_lines:    Vec<String>,
    }

    let cards: Vec<Card> = report.issues.iter().enumerate().map(|(idx, issue)| {
        let (sev_class, sev_icon) = match issue.severity {
            IssueSeverity::Critical => ("health-icon health-critical", "●"),
            IssueSeverity::Warning  => ("health-icon health-warning",  "▲"),
            IssueSeverity::Info     => ("health-icon health-info",     "ℹ"),
        };
        Card {
            idx,
            kind_label:      health_kind_label(&issue.kind),
            sev_class,
            sev_icon,
            technical_desc:  issue.technical_desc.clone(),
            business_impact: issue.business_impact.clone(),
            detail_lines:    health_detail_lines(issue),
        }
    }).collect();

    rsx! {
        div { class: "health-list",
            for card in cards.iter() {
                div { class: "health-card",
                    div { class: "health-card-header",
                        span { class: "{card.sev_class}", "{card.sev_icon}" }
                        span { class: "health-kind", "{card.kind_label}" }
                        span { class: "health-tech-desc", "{card.technical_desc}" }
                    }
                    div { class: "health-impact", "{card.business_impact}" }
                    if !card.detail_lines.is_empty() {
                        div { class: "health-detail-lines",
                            for line in card.detail_lines.iter() {
                                div { class: "health-detail-line", "{line}" }
                            }
                        }
                    }
                    div { id: "health-chart-{card.idx}", class: "health-chart" }
                }
            }
        }
    }
}

fn health_kind_label(kind: &HealthIssueKind) -> &'static str {
    match kind {
        HealthIssueKind::HeartbeatGap     => "Heartbeat Gap",
        HealthIssueKind::SequenceGap      => "Sequence Gap",
        HealthIssueKind::ExcessiveResends => "Excessive Resends",
        HealthIssueKind::Reconnect        => "Reconnect",
        HealthIssueKind::MessageRateBurst => "Message Rate Burst",
        HealthIssueKind::LateCancel       => "Late Cancel",
        HealthIssueKind::RejectedCancel   => "Rejected Cancel",
    }
}

/// Per-rule text rows shown below the business impact in each card.
fn health_detail_lines(issue: &HealthIssue) -> Vec<String> {
    match &issue.detail {
        HealthDetail::HeartbeatGaps(d) => d.gaps.iter().take(5)
            .map(|g| format!("at {}  —  {}ms", g.time, g.gap_ms))
            .collect(),
        HealthDetail::SequenceGaps(d) => d.gaps.iter().take(8)
            .map(|g| format!("{}→{}  at {}  ({} missing)", g.from_seq, g.to_seq, g.time, g.missing))
            .collect(),
        HealthDetail::Resends(d) => d.instances.iter().take(5)
            .map(|r| if r.end_seq == 0 {
                format!("at {}  —  BeginSeq={}", r.time, r.begin_seq)
            } else {
                format!("at {}  —  seq {}→{}", r.time, r.begin_seq, r.end_seq)
            })
            .collect(),
        HealthDetail::Reconnects(d) => d.logons.iter()
            .map(|l| if l.reset_seq {
                format!("{}  —  session reset  (seq={})", l.time, l.seq_num)
            } else {
                format!("{}  —  reconnect  (seq={})", l.time, l.seq_num)
            })
            .collect(),
        HealthDetail::RateBursts(d) => {
            let mut burst_buckets: Vec<(&str, usize)> = d.burst_indices.iter()
                .filter_map(|&i| d.buckets.get(i))
                .map(|b| (b.second_label.as_str(), b.count))
                .collect();
            burst_buckets.sort_by(|a, b| b.1.cmp(&a.1));
            burst_buckets.iter().take(5)
                .map(|(t, c)| format!("{}  —  {} msg/sec", t, c))
                .collect()
        },
        HealthDetail::LateCancels(d) => {
            let mut cases = d.cases.clone();
            cases.sort_by(|a, b| b.lag_ms.cmp(&a.lag_ms));
            cases.iter().take(5)
                .map(|c| format!("{}  —  {}ms after fill", c.cl_ord_id, c.lag_ms))
                .collect()
        },
        HealthDetail::RejectedCancels(d) => {
            let mut counts: std::collections::HashMap<&str, usize> =
                std::collections::HashMap::new();
            for r in &d.rejections {
                *counts.entry(r.reason_text.as_str()).or_insert(0) += 1;
            }
            let mut v: Vec<(&str, usize)> = counts.into_iter().collect();
            v.sort_by(|a, b| b.1.cmp(&a.1));
            v.iter().map(|(reason, count)| format!("{reason}: {count}")).collect()
        },
    }
}

// ── ECharts JS builders ───────────────────────────────────────────────────────

/// Build JS for the two Summary-tab donut pies: fills & rejects by counterparty.
fn build_summary_charts_js(sc: &FillQualityScorecard) -> String {
    let fill_opt   = serde_json::to_string(&summary_fill_pie(sc)).unwrap_or_default();
    let reject_opt = serde_json::to_string(&summary_reject_pie(sc)).unwrap_or_default();
    format!(r#"
(function init() {{
    if (typeof echarts === 'undefined') {{ setTimeout(init, 150); return; }}
    var fe = document.getElementById('summary-fill-pie');
    if (fe) {{
        var fc = echarts.getInstanceByDom(fe) || echarts.init(fe, null, {{renderer:'canvas'}});
        fc.setOption({fill_opt}, true);
    }}
    var re = document.getElementById('summary-reject-pie');
    if (re) {{
        var rc = echarts.getInstanceByDom(re) || echarts.init(re, null, {{renderer:'canvas'}});
        rc.setOption({reject_opt}, true);
    }}
}})();
"#, fill_opt = fill_opt, reject_opt = reject_opt)
}

/// Collapse raw `(name, value)` pairs to top-5 + "Others" bucket.
/// Input must already be sorted descending by value by the caller.
fn top5_with_others(mut pairs: Vec<(String, u64)>) -> Vec<serde_json::Value> {
    pairs.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    const MAX: usize = 5;
    if pairs.len() <= MAX {
        return pairs.iter()
            .map(|(n, v)| serde_json::json!({ "name": n, "value": v }))
            .collect();
    }
    let mut out: Vec<serde_json::Value> = pairs[..MAX].iter()
        .map(|(n, v)| serde_json::json!({ "name": n, "value": v }))
        .collect();
    let others: u64 = pairs[MAX..].iter().map(|(_, v)| v).sum();
    if others > 0 {
        out.push(serde_json::json!({ "name": "Others", "value": others }));
    }
    out
}

fn summary_fill_pie(sc: &FillQualityScorecard) -> serde_json::Value {
    let pairs: Vec<(String, u64)> = sc.rows.iter()
        .filter(|r| r.order_count > 0)
        .map(|r| {
            let fills = (r.order_count as f64 * r.fill_rate).round() as u64;
            (r.counterparty.clone(), fills)
        })
        .collect();
    summary_pie_option("Fills", top5_with_others(pairs))
}

fn summary_reject_pie(sc: &FillQualityScorecard) -> serde_json::Value {
    let pairs: Vec<(String, u64)> = sc.rows.iter()
        .filter(|r| r.order_count > 0)
        .map(|r| {
            let rejects = (r.order_count as f64 * r.reject_rate).round() as u64;
            (r.counterparty.clone(), rejects)
        })
        .collect();
    summary_pie_option("Rejects", top5_with_others(pairs))
}

fn summary_pie_option(name: &str, data: Vec<serde_json::Value>) -> serde_json::Value {
    // Legend names are the "name" fields from data.
    let legend_names: Vec<&str> = data.iter()
        .filter_map(|d| d["name"].as_str())
        .collect();
    serde_json::json!({
        "backgroundColor": "transparent",
        "tooltip": {
            "trigger": "item",
            "formatter": "{b}: {c} ({d}%)"
        },
        "legend": {
            "orient": "horizontal",
            "bottom": 2,
            "left": "center",
            "data": legend_names,
            "textStyle": { "color": "#888890", "fontSize": 10 },
            "itemWidth": 8,
            "itemHeight": 8,
            "itemGap": 8
        },
        "series": [{
            "name": name,
            "type": "pie",
            "radius": ["38%", "60%"],
            "center": ["50%", "42%"],
            "avoidLabelOverlap": false,
            "label": { "show": false },
            "labelLine": { "show": false },
            "emphasis": {
                "label": { "show": true, "fontSize": 11, "fontWeight": "bold", "color": "#dddde3" }
            },
            "data": data
        }]
    })
}

fn build_charts_js(sc: &FillQualityScorecard) -> String {
    let bar  = serde_json::to_string(&bar_option(sc)).unwrap_or_default();
    let tree = serde_json::to_string(&treemap_option(sc)).unwrap_or_default();
    format!(r#"
(function init() {{
    if (typeof echarts === 'undefined') {{ setTimeout(init, 150); return; }}
    var be = document.getElementById('fq-bar-chart');
    if (be) {{
        var b = echarts.getInstanceByDom(be) || echarts.init(be, null, {{renderer:'canvas'}});
        b.setOption({bar}, true);
    }}
    var te = document.getElementById('fq-tree-chart');
    if (te) {{
        var t = echarts.getInstanceByDom(te) || echarts.init(te, null, {{renderer:'canvas'}});
        t.setOption({tree}, true);
    }}
}})();
"#)
}

fn r1(v: f64) -> f64 { (v * 10.0).round() / 10.0 }

fn bar_option(sc: &FillQualityScorecard) -> serde_json::Value {
    // Sort worst fill-rate first so the most important rows are at top.
    let mut rows: Vec<&crate::fill_quality::ScorecardRow> = sc.rows.iter().collect();
    rows.sort_by(|a, b| a.fill_rate.partial_cmp(&b.fill_rate).unwrap_or(std::cmp::Ordering::Equal));

    let names:  Vec<&str> = rows.iter().map(|r| r.counterparty.as_str()).collect();
    let fill:   Vec<f64>  = rows.iter().map(|r| r1(r.fill_rate   * 100.0)).collect();
    let reject: Vec<f64>  = rows.iter().map(|r| r1(r.reject_rate * 100.0)).collect();
    let ack_ms: Vec<f64>  = rows.iter().map(|r| r1(r.avg_ack_ms)).collect();

    serde_json::json!({
        "backgroundColor": "transparent",
        "tooltip": { "trigger": "axis", "axisPointer": { "type": "shadow" } },
        "legend": {
            "data": ["Fill %", "Reject %", "Ack ms"],
            "textStyle": { "color": "#9ca3af" },
            "top": 4
        },
        "grid": { "left": "2%", "right": "18%", "top": "44px", "bottom": "4px", "containLabel": true },
        "xAxis": [
            {
                "type": "value", "max": 100,
                "axisLabel": { "color": "#6b7280", "formatter": "{value}%" },
                "splitLine": { "lineStyle": { "color": "#374151" } }
            },
            {
                "type": "value", "name": "ms",
                "nameTextStyle": { "color": "#6b7280" },
                "axisLabel": { "color": "#6b7280" },
                "splitLine": { "show": false }
            }
        ],
        "yAxis": {
            "type": "category",
            "data": names,
            "axisLabel": { "color": "#d1d5db", "fontSize": 11, "fontFamily": "monospace" }
        },
        "series": [
            {
                "name": "Fill %", "type": "bar", "xAxisIndex": 0,
                "data": fill,
                "itemStyle": { "color": "#4ade80", "borderRadius": [0,3,3,0] },
                "barMaxWidth": 18,
                "label": { "show": true, "position": "right", "color": "#6b7280",
                            "fontSize": 10, "formatter": "{c}%" }
            },
            {
                "name": "Reject %", "type": "bar", "xAxisIndex": 0,
                "data": reject,
                "itemStyle": { "color": "#f87171", "borderRadius": [0,3,3,0] },
                "barMaxWidth": 18,
                "label": { "show": true, "position": "right", "color": "#6b7280",
                            "fontSize": 10, "formatter": "{c}%" }
            },
            {
                "name": "Ack ms", "type": "bar", "xAxisIndex": 1,
                "data": ack_ms,
                "itemStyle": { "color": "#60a5fa", "borderRadius": [0,3,3,0] },
                "barMaxWidth": 18,
                "label": { "show": true, "position": "right", "color": "#6b7280",
                            "fontSize": 10, "formatter": "{c}ms" }
            }
        ]
    })
}

fn treemap_option(sc: &FillQualityScorecard) -> serde_json::Value {
    use std::collections::HashMap;

    let total_f = sc.rows.iter().map(|r| r.order_count).sum::<u64>().max(1) as f64;

    // Build nested map: cp → sym → bucket → count
    let mut tree: HashMap<&str, HashMap<&str, HashMap<&str, u64>>> = HashMap::new();
    for sr in &sc.size_rows {
        *tree
            .entry(sr.counterparty.as_str())
            .or_default()
            .entry(sr.symbol.as_str())
            .or_default()
            .entry(sr.bucket)
            .or_insert(0) += sr.order_count;
    }

    const BUCKET_ORDER: &[&str] = &["< 1M", "1M–5M", "5M–10M", "10M–50M", "> 50M"];

    let data: Vec<serde_json::Value> = sc.rows.iter().map(|agg| {
        let pct = agg.order_count as f64 / total_f * 100.0;
        let cp_label = format!("{} ({:.1}%)", agg.counterparty, pct);

        let sym_children: Vec<serde_json::Value> = tree
            .get(agg.counterparty.as_str())
            .map(|sym_map| {
                let mut syms: Vec<(&str, u64)> = sym_map.iter()
                    .map(|(sym, buckets)| (*sym, buckets.values().sum()))
                    .collect();
                syms.sort_by(|a, b| b.1.cmp(&a.1));

                syms.iter().map(|(sym, sym_total)| {
                    let bucket_children: Vec<serde_json::Value> = BUCKET_ORDER.iter()
                        .filter_map(|&b| {
                            sym_map.get(sym)
                                .and_then(|bm| bm.get(b))
                                .map(|&cnt| serde_json::json!({ "name": b, "value": cnt }))
                        })
                        .collect();
                    serde_json::json!({
                        "name":     sym,
                        "value":    sym_total,
                        "children": bucket_children
                    })
                }).collect()
            })
            .unwrap_or_default();

        serde_json::json!({
            "name":     cp_label,
            "value":    agg.order_count,
            "children": sym_children
        })
    }).collect();

    serde_json::json!({
        "backgroundColor": "transparent",
        "tooltip": { "formatter": "{b}: {c} orders" },
        "series": [{
            "type": "treemap",
            "width": "100%", "height": "100%",
            "roam": false,
            "nodeClick": "zoomToNode",
            "visibleMin": 100,
            "label": {
                "show": true,
                "color": "#fff",
                "fontSize": 11,
                "overflow": "truncate"
            },
            "upperLabel": {
                "show": true,
                "height": 26,
                "color": "#fff",
                "fontWeight": "bold"
            },
            "breadcrumb": {
                "show": true,
                "height": 28,
                "itemStyle": { "color": "#1e293b", "shadowBlur": 0 },
                "textStyle": { "color": "#e5e7eb" }
            },
            "emphasis": { "focus": "descendant" },
            "levels": [
                {
                    "itemStyle": {
                        "borderWidth": 6,
                        "borderColor": "#0f172a",
                        "gapWidth": 6
                    },
                    "upperLabel": {
                        "show": true,
                        "height": 28,
                        "fontSize": 13,
                        "fontWeight": "bold",
                        "color": "#fff"
                    }
                },
                {
                    "colorSaturation": [0.35, 0.65],
                    "itemStyle": {
                        "borderWidth": 3,
                        "gapWidth": 3,
                        "borderColorSaturation": 0.6
                    },
                    "upperLabel": {
                        "show": true,
                        "height": 20,
                        "fontSize": 11,
                        "color": "#fff"
                    }
                },
                {
                    "colorSaturation": [0.25, 0.55],
                    "colorAlpha": [0.65, 1.0],
                    "itemStyle": {
                        "borderWidth": 1,
                        "gapWidth": 1,
                        "borderColorSaturation": 0.5
                    },
                    "label": {
                        "show": true,
                        "fontSize": 10,
                        "color": "#fff"
                    }
                }
            ],
            "data": data
        }]
    })
}

// ── Health tab ECharts builders ───────────────────────────────────────────────

fn build_health_charts_js(report: &SessionHealthReport) -> String {
    let body: Vec<String> = report.issues.iter().enumerate()
        .filter_map(|(idx, issue)| health_issue_chart_js(idx, issue))
        .collect();
    if body.is_empty() { return String::new(); }
    format!(
        "(function init() {{\n  \
         if (typeof echarts === 'undefined') {{ setTimeout(init, 150); return; }}\n  \
         {body}\n}})();\n",
        body = body.join("\n  ")
    )
}

fn health_issue_chart_js(idx: usize, issue: &HealthIssue) -> Option<String> {
    let opt = match &issue.detail {
        HealthDetail::HeartbeatGaps(d)   => health_hb_chart(d),
        HealthDetail::SequenceGaps(d)    => health_seq_chart(d),
        HealthDetail::Resends(d)         => health_resend_chart(d),
        HealthDetail::Reconnects(d)      => health_reconnect_chart(d),
        HealthDetail::RateBursts(d)      => health_burst_chart(d),
        HealthDetail::LateCancels(d)     => health_late_cancel_chart(d),
        HealthDetail::RejectedCancels(d) => health_rejected_cancel_chart(d),
    };
    let opt_json = serde_json::to_string(&opt).ok()?;
    Some(format!(
        "var e{idx}=document.getElementById('health-chart-{idx}');\
         if(e{idx}){{var c{idx}=echarts.getInstanceByDom(e{idx})||\
         echarts.init(e{idx},null,{{renderer:'canvas'}});\
         c{idx}.setOption({opt_json},true);}}",
        idx = idx, opt_json = opt_json
    ))
}

// Rule 1 — Heartbeat Gap: scatter X=time, Y=gap_ms, threshold markLine.
fn health_hb_chart(d: &HeartbeatGapDetail) -> serde_json::Value {
    let labels: Vec<&str> = d.gaps.iter().map(|g| g.time.as_str()).collect();
    let values: Vec<i64>  = d.gaps.iter().map(|g| g.gap_ms).collect();
    let threshold_ms      = d.configured_interval_sec * 1_500;
    serde_json::json!({
        "backgroundColor": "transparent",
        "tooltip": { "trigger": "axis" },
        "grid": { "left": "3%", "right": "3%", "top": "14px", "bottom": "40px", "containLabel": true },
        "xAxis": { "type": "category", "data": labels,
            "axisLabel": { "color": "#6b7280", "fontSize": 10, "rotate": 30 } },
        "yAxis": { "type": "value",
            "axisLabel": { "color": "#6b7280", "formatter": "{value}ms" },
            "splitLine": { "lineStyle": { "color": "#374151" } } },
        "series": [{
            "type": "scatter", "data": values,
            "symbolSize": 9, "itemStyle": { "color": "#b8922a" },
            "markLine": {
                "silent": true,
                "lineStyle": { "color": "#b8922a", "type": "dashed", "opacity": 0.5 },
                "label": { "formatter": "threshold", "color": "#b8922a", "fontSize": 10 },
                "data": [{ "yAxis": threshold_ms }]
            }
        }]
    })
}

// Rule 2 — Sequence Gap: bar X=gap label, Y=missing count.
fn health_seq_chart(d: &SequenceGapDetail) -> serde_json::Value {
    let labels: Vec<String> = d.gaps.iter()
        .map(|g| format!("{}→{}", g.from_seq, g.to_seq))
        .collect();
    let values: Vec<u64> = d.gaps.iter().map(|g| g.missing).collect();
    serde_json::json!({
        "backgroundColor": "transparent",
        "tooltip": { "trigger": "axis", "formatter": "{b}<br/>Missing: {c}" },
        "grid": { "left": "3%", "right": "3%", "top": "14px", "bottom": "40px", "containLabel": true },
        "xAxis": { "type": "category", "data": labels,
            "axisLabel": { "color": "#6b7280", "fontSize": 10, "rotate": 30 } },
        "yAxis": { "type": "value",
            "axisLabel": { "color": "#6b7280" },
            "splitLine": { "lineStyle": { "color": "#374151" } } },
        "series": [{
            "type": "bar", "data": values, "barMaxWidth": 40,
            "itemStyle": { "color": "#f87171", "borderRadius": [3,3,0,0] },
            "label": { "show": true, "position": "top", "color": "#9ca3af", "fontSize": 10 }
        }]
    })
}

// Rule 3 — Excessive Resends: scatter timeline, Y=range size requested.
fn health_resend_chart(d: &ResendDetail) -> serde_json::Value {
    let labels: Vec<&str> = d.instances.iter().map(|r| r.time.as_str()).collect();
    let values: Vec<u64>  = d.instances.iter()
        .map(|r| if r.end_seq > r.begin_seq { r.end_seq - r.begin_seq + 1 } else { 1 })
        .collect();
    serde_json::json!({
        "backgroundColor": "transparent",
        "tooltip": { "trigger": "axis", "formatter": "{b}<br/>Range: {c} messages" },
        "grid": { "left": "3%", "right": "3%", "top": "14px", "bottom": "40px", "containLabel": true },
        "xAxis": { "type": "category", "data": labels,
            "axisLabel": { "color": "#6b7280", "fontSize": 10, "rotate": 30 } },
        "yAxis": { "type": "value", "name": "range",
            "axisLabel": { "color": "#6b7280" },
            "splitLine": { "lineStyle": { "color": "#374151" } } },
        "series": [{
            "type": "scatter", "data": values,
            "symbolSize": 9, "itemStyle": { "color": "#fb923c" }
        }]
    })
}

// Rule 4 — Reconnects: bar showing gap in seconds before each reconnect logon.
// Amber = mid-session reconnect; grey = clean session reset.
fn health_reconnect_chart(d: &ReconnectDetail) -> serde_json::Value {
    if d.logons.len() < 2 { return serde_json::json!({}); }
    let mut labels:   Vec<String>            = Vec::new();
    let mut gap_data: Vec<serde_json::Value> = Vec::new();
    for window in d.logons.windows(2) {
        let gap_sec = parse_time_us(&window[1].time)
            .zip(parse_time_us(&window[0].time))
            .map(|(c, p)| ((c - p).abs() / 1_000_000) as i64)
            .unwrap_or(0);
        labels.push(window[1].time.clone());
        let color = if window[1].reset_seq { "#6b7280" } else { "#b8922a" };
        gap_data.push(serde_json::json!({ "value": gap_sec, "itemStyle": { "color": color } }));
    }
    serde_json::json!({
        "backgroundColor": "transparent",
        "tooltip": { "trigger": "axis", "formatter": "{b}<br/>Gap before logon: {c}s" },
        "grid": { "left": "3%", "right": "3%", "top": "14px", "bottom": "40px", "containLabel": true },
        "xAxis": { "type": "category", "data": labels,
            "axisLabel": { "color": "#6b7280", "fontSize": 10, "rotate": 30 } },
        "yAxis": { "type": "value", "name": "sec",
            "axisLabel": { "color": "#6b7280", "formatter": "{value}s" },
            "splitLine": { "lineStyle": { "color": "#374151" } } },
        "series": [{
            "type": "bar", "data": gap_data, "barMaxWidth": 30,
            "label": { "show": true, "position": "top",
                       "formatter": "{c}s", "color": "#9ca3af", "fontSize": 10 }
        }]
    })
}

// Rule 5 — Rate Burst: bar chart of msg/sec, down-sampled to ≤200 points, threshold line.
fn health_burst_chart(d: &RateBurstDetail) -> serde_json::Value {
    const MAX_POINTS: usize = 200;
    let (labels, counts): (Vec<String>, Vec<usize>) = if d.buckets.len() <= MAX_POINTS {
        (
            d.buckets.iter().map(|b| b.second_label.clone()).collect(),
            d.buckets.iter().map(|b| b.count).collect(),
        )
    } else {
        let group = (d.buckets.len() + MAX_POINTS - 1) / MAX_POINTS;
        let mut lbs = Vec::new();
        let mut cts = Vec::new();
        for chunk in d.buckets.chunks(group) {
            lbs.push(chunk[0].second_label.clone());
            cts.push(chunk.iter().map(|b| b.count).max().unwrap_or(0));
        }
        (lbs, cts)
    };
    let threshold = d.threshold;
    let data: Vec<serde_json::Value> = counts.iter().map(|&c| {
        if c > threshold {
            serde_json::json!({ "value": c, "itemStyle": { "color": "#b8922a" } })
        } else {
            serde_json::json!(c)
        }
    }).collect();
    serde_json::json!({
        "backgroundColor": "transparent",
        "tooltip": { "trigger": "axis" },
        "grid": { "left": "3%", "right": "3%", "top": "14px", "bottom": "40px", "containLabel": true },
        "xAxis": { "type": "category", "data": labels,
            "axisLabel": { "color": "#6b7280", "fontSize": 10 },
            "axisTick": { "show": false } },
        "yAxis": { "type": "value",
            "axisLabel": { "color": "#6b7280" },
            "splitLine": { "lineStyle": { "color": "#374151" } } },
        "series": [{
            "type": "bar", "data": data, "barMaxWidth": 6,
            "itemStyle": { "color": "#4b5563", "borderRadius": [1,1,0,0] },
            "markLine": {
                "silent": true,
                "lineStyle": { "color": "#b8922a", "type": "dashed", "opacity": 0.7 },
                "label": { "formatter": "threshold", "color": "#b8922a", "fontSize": 10 },
                "data": [{ "yAxis": threshold }]
            }
        }]
    })
}

// Rule 6 — Late Cancels: scatter X=cancel time, Y=lag ms.
fn health_late_cancel_chart(d: &LateCancelDetail) -> serde_json::Value {
    let labels: Vec<&str> = d.cases.iter().map(|c| c.cancel_time.as_str()).collect();
    let values: Vec<i64>  = d.cases.iter().map(|c| c.lag_ms).collect();
    serde_json::json!({
        "backgroundColor": "transparent",
        "tooltip": { "trigger": "axis", "formatter": "{b}<br/>Lag: {c}ms" },
        "grid": { "left": "3%", "right": "3%", "top": "14px", "bottom": "40px", "containLabel": true },
        "xAxis": { "type": "category", "data": labels,
            "axisLabel": { "color": "#6b7280", "fontSize": 10, "rotate": 30 } },
        "yAxis": { "type": "value",
            "axisLabel": { "color": "#6b7280", "formatter": "{value}ms" },
            "splitLine": { "lineStyle": { "color": "#374151" } } },
        "series": [{
            "type": "scatter", "data": values,
            "symbolSize": 9, "itemStyle": { "color": "#fb923c" }
        }]
    })
}

// Rule 7 — Rejected Cancels: donut pie by rejection reason (tag 102).
fn health_rejected_cancel_chart(d: &RejectedCancelDetail) -> serde_json::Value {
    let mut counts: std::collections::HashMap<&str, u64> = std::collections::HashMap::new();
    for r in &d.rejections {
        *counts.entry(r.reason_text.as_str()).or_insert(0) += 1;
    }
    let mut pairs: Vec<(&str, u64)> = counts.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));
    let data: Vec<serde_json::Value> = pairs.iter()
        .map(|(name, value)| serde_json::json!({ "name": name, "value": value }))
        .collect();
    serde_json::json!({
        "backgroundColor": "transparent",
        "tooltip": { "trigger": "item", "formatter": "{b}: {c} ({d}%)" },
        "legend": {
            "orient": "horizontal", "bottom": 2, "left": "center",
            "textStyle": { "color": "#888890", "fontSize": 10 },
            "itemWidth": 8, "itemHeight": 8
        },
        "series": [{
            "type": "pie", "radius": ["38%", "60%"], "center": ["50%", "42%"],
            "label": { "show": false }, "labelLine": { "show": false },
            "emphasis": {
                "label": { "show": true, "fontSize": 11, "fontWeight": "bold", "color": "#dddde3" }
            },
            "data": data
        }]
    })
}
