//! Overview Session Analysis — three-tab panel: Summary, Fill Quality, Health.

use dioxus::prelude::*;
use dioxus::document::eval;

use crate::export::{messages_to_csv, now_tag};
use crate::fill_quality::{build_scorecard, FillQualityScorecard, ScorecardRow};
use crate::model::FixMessage;
use crate::session_health::{HealthIssueKind, IssueSeverity, SessionHealthReport};
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
pub fn overview_panel(messages: Signal<Vec<FixMessage>>, pro: bool) -> Element {
    let mut active_tab          = use_signal(|| OverviewTab::Summary);
    let sort_col                = use_signal(|| SortCol::Orders);
    let sort_asc                = use_signal(|| false);
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

    // Draw / redraw ECharts whenever the user switches to Charts view or data arrives.
    use_effect(move || {
        let tab  = active_tab.read().clone();
        let view = fq_view.read().clone();
        let maybe_js = {
            let data_ref = computed.read();
            if tab == OverviewTab::FillQuality && view == FqView::Charts {
                data_ref.as_ref().map(|d| build_charts_js(&d.scorecard))
            } else {
                None
            }
        };
        if let Some(js) = maybe_js {
            spawn(async move { let _ = eval(&js).await; });
        }
    });

    let tab_val   = active_tab.read().clone();
    let drill_val = drill_counterparty.read().clone();
    let data_opt: Option<OverviewData> = computed.read().clone();

    rsx! {
        div { class: "overview-panel",

            // ── Header ───────────────────────────────────────────────────────
            div { class: "overview-header",
                div { class: "overview-header-left",
                    h2 { class: "overview-title", "Session Analysis" }
                    if let Some(ref d) = data_opt {
                        span { class: "overview-meta",
                            "{d.summary.sender} → {d.summary.target}  ·  \
                            {d.summary.start_time} – {d.summary.end_time}  ·  \
                            {d.summary.total_messages} messages"
                        }
                    }
                }
                if pro {
                    div { class: "overview-header-actions",
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
                        OverviewTab::Summary     => render_summary(&data.summary),
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

fn render_summary(s: &SessionSummary) -> Element {
    let stats = &s.order_stats;
    let lats  = &s.latency_stats;

    rsx! {
        div { class: "summary-body",

            div { class: "summary-section",
                div { class: "summary-row",
                    span { class: "summary-label", "Session" }
                    span { class: "summary-value summary-session-label",
                        "{s.session_label}"
                    }
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
                            span { class: "summary-duration",
                                "  ({s.duration_str})"
                            }
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
                        span { class: "summary-pct summary-pct-green",
                            "  ({stats.fill_pct:.1}%)"
                        }
                    }
                }
                div { class: "summary-row summary-sub",
                    span { class: "summary-label", "  Cancelled" }
                    span { class: "summary-value",
                        "{stats.cancelled}"
                        span { class: "summary-pct",
                            "  ({stats.cancel_pct:.1}%)"
                        }
                    }
                }
                div { class: "summary-row summary-sub",
                    span { class: "summary-label", "  Rejected" }
                    span { class: "summary-value",
                        "{stats.rejected}"
                        span {
                            class: if stats.rejected > 0
                                { "summary-pct summary-pct-warn" } else { "summary-pct" },
                            "  ({stats.reject_pct:.1}%)"
                        }
                    }
                }
            }

            div { class: "summary-divider" }

            div { class: "summary-section",
                div { class: "summary-row",
                    span { class: "summary-label", "Avg ack latency" }
                    span { class: "summary-value summary-mono",
                        "{lats.avg_ack_ms:.2}ms"
                    }
                }
                div { class: "summary-row",
                    span { class: "summary-label", "Avg fill latency" }
                    span { class: "summary-value summary-mono",
                        "{lats.avg_fill_ms:.1}ms"
                    }
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
    rsx! {
        div { class: "health-list",
            for issue in report.issues.iter() {
                div { class: "health-issue",
                    div { class: "health-issue-header",
                        span {
                            class: match issue.severity {
                                IssueSeverity::Critical => "health-icon health-critical",
                                IssueSeverity::Warning  => "health-icon health-warning",
                                IssueSeverity::Info     => "health-icon health-info",
                            },
                            {match issue.severity {
                                IssueSeverity::Critical => "●",
                                IssueSeverity::Warning  => "▲",
                                IssueSeverity::Info     => "ℹ",
                            }}
                        }
                        span { class: "health-kind",
                            {health_kind_label(&issue.kind)}
                        }
                        if !issue.time.is_empty() {
                            span { class: "health-time", "{issue.time}" }
                        }
                        if !issue.msg_indices.is_empty() {
                            span { class: "health-msg-count",
                                {format!("({} msg)", issue.msg_indices.len())}
                            }
                        }
                    }
                    div { class: "health-tech-desc", "{issue.technical_desc}" }
                    div { class: "health-impact",    "{issue.business_impact}" }
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

// ── ECharts JS builders ───────────────────────────────────────────────────────

/// Build the JavaScript snippet that initialises/updates both ECharts instances.
/// Data is serialised via serde_json so all strings are properly escaped.
fn build_charts_js(sc: &FillQualityScorecard) -> String {
    let bar  = serde_json::to_string(&bar_option(sc)).unwrap_or_default();
    let tree = serde_json::to_string(&treemap_option(sc)).unwrap_or_default();
    // `{{` / `}}` → literal `{` / `}` in format output; `{bar}` / `{tree}` are injected.
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
"#, bar = bar, tree = tree)
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
