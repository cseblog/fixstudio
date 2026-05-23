//! "Now" dashboard — single-screen current-state view.
//!
//! Lives above Timeline / Latency / Session / Validator in the view-tab
//! strip. Renders one row of headline KPIs + the heartbeat grid + a
//! per-LP traffic share table. Everything is computed in pure functions
//! so the component itself stays trivially testable by inspection.

use dioxus::prelude::*;

use crate::live_health::{self, HbStatus, HeartbeatRow};
use crate::model::FixMessage;
use crate::now_metrics::{self, NowSnapshot};

#[component]
pub fn now_panel(messages: Signal<Vec<FixMessage>>) -> Element {
    let snap = use_memo(move || now_metrics::compute(&messages.read()));
    let hb   = use_memo(move || live_health::compute(&messages.read()));

    let s: NowSnapshot = snap.read().clone();
    let hb_rows: Vec<HeartbeatRow> = hb.read().clone();

    rsx! {
        div { class: "now-panel",

            // ── Headline KPIs ───────────────────────────────────────────
            div { class: "now-kpis",
                Kpi {
                    label: "Open orders",
                    value: format!("{}", s.open_orders),
                    sub:   "since session start",
                    tone:  "neutral".to_string(),
                }
                Kpi {
                    label: format!("Reject rate · last {}s", s.window_secs),
                    value: format!("{:.1}%", s.window_reject_pct),
                    sub:   format!("{} of {} msgs", s.window_rejects, s.window_messages),
                    tone:  reject_tone(s.window_reject_pct),
                }
                Kpi {
                    label: format!("p50 ack · last {}s", s.window_secs),
                    value: fmt_ms(s.window_ack_p50_ms),
                    sub:   format!("{} samples", s.window_ack_count),
                    tone:  latency_tone(s.window_ack_p50_ms),
                }
                Kpi {
                    label: format!("p95 ack · last {}s", s.window_secs),
                    value: fmt_ms(s.window_ack_p95_ms),
                    sub:   format!("{} samples", s.window_ack_count),
                    tone:  latency_tone(s.window_ack_p95_ms / 5.0),  // looser cutoff at p95
                }
            }

            // ── Heartbeat grid ──────────────────────────────────────────
            section { class: "now-section",
                h3 { class: "now-section-title", "Sessions" }
                if hb_rows.is_empty() {
                    div { class: "now-empty", "No sessions seen yet." }
                } else {
                    div { class: "now-hb-grid",
                        for r in hb_rows.iter() {
                            {
                                let cls = match r.status {
                                    HbStatus::Fresh => "now-hb-card now-hb-fresh",
                                    HbStatus::Stale => "now-hb-card now-hb-stale",
                                    HbStatus::Dead  => "now-hb-card now-hb-dead",
                                };
                                let dot_cls = match r.status {
                                    HbStatus::Fresh => "now-hb-dot now-hb-dot-fresh",
                                    HbStatus::Stale => "now-hb-dot now-hb-dot-stale",
                                    HbStatus::Dead  => "now-hb-dot now-hb-dot-dead",
                                };
                                let age = live_health::fmt_age(r.last_msg_age_us);
                                let status_text = match r.status {
                                    HbStatus::Fresh => "FRESH",
                                    HbStatus::Stale => "STALE",
                                    HbStatus::Dead  => if r.closed { "LOGGED OUT" } else { "DEAD" },
                                };
                                rsx! {
                                    div { class: "{cls}",
                                        div { class: "now-hb-head",
                                            span { class: "{dot_cls}" }
                                            span { class: "now-hb-name", "{r.sender}→{r.target}" }
                                        }
                                        div { class: "now-hb-status", "{status_text}" }
                                        div { class: "now-hb-meta",
                                            span { "last msg " }
                                            strong { "{age}" }
                                            span { " · HB {r.interval_secs}s" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── Top LPs by traffic in window ───────────────────────────
            section { class: "now-section",
                h3 { class: "now-section-title", "Top traffic (last {s.window_secs}s)" }
                if s.top_lps.is_empty() {
                    div { class: "now-empty", "No traffic in window." }
                } else {
                    div { class: "now-lp-list",
                        for l in s.top_lps.iter() {
                            div { class: "now-lp-row",
                                span { class: "now-lp-name", "{l.lp}" }
                                div { class: "now-lp-bar-wrap",
                                    div {
                                        class: "now-lp-bar",
                                        style: "width: {l.pct}%;",
                                    }
                                }
                                span { class: "now-lp-count", "{l.count}" }
                                span { class: "now-lp-pct", "{l.pct:.1}%" }
                            }
                        }
                    }
                }
            }

            // ── Footer: clock reference ────────────────────────────────
            div { class: "now-footer",
                "Reference clock: latest msg @ "
                strong { "{s.now_label}" }
                " · {s.total_messages} total messages"
            }
        }
    }
}

#[component]
fn Kpi(label: String, value: String, sub: String, tone: String) -> Element {
    let cls = format!("now-kpi now-kpi-{tone}");
    rsx! {
        div { class: "{cls}",
            div { class: "now-kpi-label", "{label}" }
            div { class: "now-kpi-value", "{value}" }
            div { class: "now-kpi-sub",   "{sub}" }
        }
    }
}

fn fmt_ms(v: f64) -> String {
    if v >= 1000.0      { format!("{:.2} s", v / 1000.0) }
    else if v >= 10.0   { format!("{:.0} ms", v) }
    else if v > 0.0     { format!("{:.2} ms", v) }
    else                { "—".to_string() }
}

fn reject_tone(pct: f64) -> String {
    if pct >= 5.0 { "bad".into() }
    else if pct >= 1.0 { "warn".into() }
    else { "good".into() }
}

fn latency_tone(ms: f64) -> String {
    if ms >= 50.0 { "bad".into() }
    else if ms >= 10.0 { "warn".into() }
    else if ms > 0.0 { "good".into() }
    else { "neutral".into() }
}
