use dioxus::prelude::*;

use crate::dictionary::badge_class;
use crate::model::FixMessage;

fn build_detail_text(m: &FixMessage) -> String {
    let mut parts = Vec::new();
    if !m.side.is_empty() { parts.push(m.side.clone()); }
    if !m.order_qty.is_empty() { parts.push(m.order_qty.clone()); }
    if !m.symbol.is_empty() { parts.push(m.symbol.clone()); }
    if !m.text.is_empty() { parts.push(m.text.clone()); }
    parts.join("  ")
}

/// Case-insensitive substring match; empty filter means "show all".
fn col_match(value: &str, filter: &str) -> bool {
    filter.is_empty() || value.to_lowercase().contains(filter)
}

/// Format a microsecond duration adaptively: µs / ms / s.
fn format_duration(us: u64) -> String {
    if us < 1_000 {
        format!("{us}µs")
    } else if us < 1_000_000 {
        format!("{:.1}ms", us as f64 / 1_000.0)
    } else {
        format!("{:.2}s", us as f64 / 1_000_000.0)
    }
}

/// Renders the Timeline panel (left side).
#[component]
pub fn timeline_panel(
    messages: Signal<Vec<FixMessage>>,
    selected_idx: Signal<Option<usize>>,
    skip_heartbeats: Signal<bool>,
    parse_stats: Signal<Option<(usize, u64)>>,
) -> Element {
    // Per-column filter state (stored as lowercase so matching is cheap)
    let mut f_time   = use_signal(String::new);
    let mut f_sender = use_signal(String::new);
    let mut f_target = use_signal(String::new);
    let mut f_msg    = use_signal(String::new);
    let mut f_clord  = use_signal(String::new);
    let mut f_detail = use_signal(String::new);

    let skip_hb = *skip_heartbeats.read();
    let msgs    = messages.read();
    let sel     = *selected_idx.read();

    // Pre-lowercase once so every row comparison is fast
    let ft  = f_time.read().to_lowercase();
    let fs  = f_sender.read().to_lowercase();
    let fta = f_target.read().to_lowercase();
    let fm  = f_msg.read().to_lowercase();
    let fc  = f_clord.read().to_lowercase();
    let fd  = f_detail.read().to_lowercase();

    let has_filter = !ft.is_empty() || !fs.is_empty() || !fta.is_empty()
        || !fm.is_empty() || !fc.is_empty() || !fd.is_empty();

    let timeline_indices: Vec<usize> = msgs
        .iter()
        .enumerate()
        .filter(|(_, m)| !(skip_hb && (m.msg_type_raw == "0" || m.msg_type_raw == "A")))
        .filter(|(_, m)| {
            let detail = build_detail_text(m);
            col_match(&m.time, &ft)
                && col_match(&m.sender, &fs)
                && col_match(&m.target, &fta)
                && col_match(m.msg_type_label, &fm)
                && col_match(&m.cl_ord_id, &fc)
                && col_match(&detail, &fd)
        })
        .map(|(i, _)| i)
        .collect();

    rsx! {
        div { class: "panel-timeline",
            div { class: "panel-header",
                div { class: "panel-title",
                    h2 { "Timeline" }
                    if let Some((count, us)) = *parse_stats.read() {
                        span { class: "parse-stats", "parsed {count} messages in {format_duration(us)}" }
                    }
                }
                div { class: "header-actions",
                    if has_filter {
                        button {
                            class: "btn-clear-filter",
                            onclick: move |_| {
                                f_time.set(String::new());
                                f_sender.set(String::new());
                                f_target.set(String::new());
                                f_msg.set(String::new());
                                f_clord.set(String::new());
                                f_detail.set(String::new());
                            },
                            "✕ clear filters"
                        }
                    }
                    label { class: "check-label",
                        input {
                            r#type: "checkbox",
                            checked: *skip_heartbeats.read(),
                            onchange: move |evt: Event<FormData>| skip_heartbeats.set(evt.checked()),
                        }
                        " Skip heartbeats"
                    }
                }
            }

            div { class: "table-wrap",
                // ── Column headers ──
                div { class: "tbl-header tbl-timeline-row",
                    span { "Time" }
                    span { "Sender" }
                    span { "Target" }
                    span { "Message" }
                    span { "Client order ID" }
                    span { "Detail" }
                }
                // ── Per-column filter inputs (same grid, sits just below headers) ──
                div { class: "tbl-filter tbl-timeline-row",
                    input { class: "col-filter", placeholder: "filter…",
                        value: "{f_time.read()}",
                        oninput: move |e| f_time.set(e.value()),
                    }
                    input { class: "col-filter", placeholder: "filter…",
                        value: "{f_sender.read()}",
                        oninput: move |e| f_sender.set(e.value()),
                    }
                    input { class: "col-filter", placeholder: "filter…",
                        value: "{f_target.read()}",
                        oninput: move |e| f_target.set(e.value()),
                    }
                    input { class: "col-filter", placeholder: "filter…",
                        value: "{f_msg.read()}",
                        oninput: move |e| f_msg.set(e.value()),
                    }
                    input { class: "col-filter", placeholder: "filter…",
                        value: "{f_clord.read()}",
                        oninput: move |e| f_clord.set(e.value()),
                    }
                    input { class: "col-filter", placeholder: "filter…",
                        value: "{f_detail.read()}",
                        oninput: move |e| f_detail.set(e.value()),
                    }
                }
                // ── Rows ──
                div { class: "tbl-body",
                    for idx in timeline_indices.iter().copied() {
                        div {
                            class: if sel == Some(idx) { "tbl-row tbl-timeline-row row-selected" } else { "tbl-row tbl-timeline-row" },
                            onclick: move |_| selected_idx.set(Some(idx)),
                            span { class: "cell-time", "{msgs[idx].time}" }
                            span { "{msgs[idx].sender}" }
                            span { "{msgs[idx].target}" }
                            span { span { class: "badge {badge_class(&msgs[idx].msg_type_raw)}", "{msgs[idx].msg_type_label}" } }
                            span { "{msgs[idx].cl_ord_id}" }
                            span { class: "cell-detail", "{build_detail_text(&msgs[idx])}" }
                        }
                    }
                    if timeline_indices.is_empty() {
                        div { class: "empty-state",
                            if has_filter { "No messages match the current filters." }
                            else { "No messages parsed yet." }
                        }
                    }
                }
            }
        }
    }
}
