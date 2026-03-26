use dioxus::prelude::*;
use dioxus::document::eval;

use crate::dictionary::badge_class;
use crate::export::{messages_to_csv, now_tag};
use crate::model::FixMessage;

const INITIAL_DISPLAY: usize = 1000;
const LOAD_MORE: usize = 1000;

fn build_detail_text(m: &FixMessage) -> String {
    let mut parts = Vec::new();
    if !m.side.is_empty()      { parts.push(m.side.clone()); }
    if !m.order_qty.is_empty() { parts.push(m.order_qty.clone()); }
    if !m.symbol.is_empty()    { parts.push(m.symbol.clone()); }
    if !m.text.is_empty()      { parts.push(m.text.clone()); }
    parts.join("  ")
}

/// Case-insensitive substring match; `filter` must already be ASCII-lowercased.
/// Zero-allocation: avoids the heap alloc from `to_ascii_lowercase()` on every row.
fn col_match(value: &str, filter: &str) -> bool {
    if filter.is_empty() { return true; }
    let fb = filter.as_bytes();
    if fb.len() > value.len() { return false; }
    value.as_bytes().windows(fb.len())
        .any(|w| w.iter().zip(fb).all(|(&a, &b)| a.to_ascii_lowercase() == b))
}

/// Time filter: supports `>=` / `<=` prefixes for range queries or plain substring match.
/// Timestamps are `YYYY-MM-DD HH:MM:SS` (pure ASCII digits/punctuation) so both
/// lexicographic ordering and case-insensitive substring match work without allocation.
fn time_match(time: &str, filter: &str) -> bool {
    if filter.is_empty() { return true; }
    if let Some(t) = filter.strip_prefix(">=") { return time >= t.trim(); }
    if let Some(t) = filter.strip_prefix("<=") { return time <= t.trim(); }
    time.contains(filter)
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

#[component]
pub fn timeline_panel(
    messages: Signal<Vec<FixMessage>>,
    selected_idx: Signal<Option<usize>>,
    skip_heartbeats: Signal<bool>,
    parse_stats: Signal<Option<(usize, u64)>>,
    pro: bool,
) -> Element {
    let mut f_time    = use_signal(String::new);
    let mut f_time_op = use_signal(|| String::from("="));  // "=" | ">=" | "<="
    let mut f_sender = use_signal(String::new);
    let mut f_target = use_signal(String::new);
    let mut f_msg    = use_signal(String::new);
    let mut f_clord  = use_signal(String::new);
    let mut f_detail = use_signal(String::new);

    let mut display_limit = use_signal(|| INITIAL_DISPLAY);

    // Installs a scroll listener on #timeline-scroll once on mount.
    // When within 200px of the bottom and the .cap-notice sentinel exists,
    // JS sends true → Rust increments display_limit.
    use_effect(move || {
        let mut dl = display_limit;
        spawn(async move {
            let mut e = eval(r#"
                (function() {
                    var cooldown = false;
                    var attached = false;
                    function attach() {
                        if (attached) return;
                        var el = document.getElementById('timeline-scroll');
                        if (!el) { setTimeout(attach, 100); return; }
                        attached = true;
                        el.addEventListener('scroll', function() {
                            if (cooldown) return;
                            if (!el.querySelector('.cap-notice')) return;
                            var dist = el.scrollHeight - el.scrollTop - el.clientHeight;
                            if (dist < 200) {
                                cooldown = true;
                                setTimeout(function() { cooldown = false; }, 400);
                                dioxus.send(true);
                            }
                        });
                    }
                    attach();
                })();
            "#);
            loop {
                match e.recv::<bool>().await {
                    Ok(true)  => { let cur = *dl.read(); dl.set(cur + LOAD_MORE); }
                    Ok(false) => {}
                    Err(_)    => break,
                }
            }
        });
    });

    // Reset display_limit and scroll position whenever filters or data change.
    use_effect(move || {
        f_time.read();
        f_time_op.read();
        f_sender.read();
        f_target.read();
        f_msg.read();
        f_clord.read();
        f_detail.read();
        skip_heartbeats.read();
        let _ = messages.read().len();
        display_limit.set(INITIAL_DISPLAY);
        eval("var el = document.getElementById('timeline-scroll'); if (el) el.scrollTop = 0;");
    });

    // Memoized: re-computes only when filters, skip_heartbeats, or messages change.
    // Scrolling (display_limit changes) does NOT trigger a re-scan.
    let timeline_indices = use_memo(move || -> Vec<usize> {
        let skip_hb = *skip_heartbeats.read();
        let msgs    = messages.read();
        let ft_val = f_time.read().clone();
        let ft_op  = f_time_op.read().clone();
        // Combine operator + value into the format time_match expects:
        //   "="  → plain substring match; ">="/"<=" → prefixed range query
        let ft_raw = if ft_op == "=" || ft_val.is_empty() {
            ft_val
        } else {
            format!("{}{}", ft_op, ft_val)
        };
        let fs      = f_sender.read().to_ascii_lowercase();
        let fta     = f_target.read().to_ascii_lowercase();
        let fm      = f_msg.read().to_ascii_lowercase();
        let fc      = f_clord.read().to_ascii_lowercase();
        let fd      = f_detail.read().to_ascii_lowercase();

        let mut indices: Vec<usize> = msgs.iter()
            .enumerate()
            .filter(|(_, m)| !(skip_hb && (m.msg_type_raw == "0" || m.msg_type_raw == "A")))
            .filter(|(_, m)| {
                time_match(&m.time, &ft_raw)
                    && col_match(&m.sender, &fs)
                    && col_match(&m.target, &fta)
                    && col_match(m.msg_type_label, &fm)
                    && (fc.is_empty()
                        || col_match(&m.cl_ord_id,    &fc)
                        || col_match(&m.quote_id,     &fc)
                        || col_match(&m.quote_req_id, &fc))
                    && (fd.is_empty() || col_match(&build_detail_text(m), &fd))
            })
            .map(|(i, _)| i)
            .collect();
        indices.sort_unstable_by(|&a, &b| msgs[b].time.cmp(&msgs[a].time));
        indices
    });

    let msgs = messages.read();
    let sel  = *selected_idx.read();

    let ft_val = f_time.read().clone();
    let ft_op  = f_time_op.read().clone();

    let has_filter = !ft_val.is_empty()
        || !f_sender.read().is_empty()
        || !f_target.read().is_empty()
        || !f_msg.read().is_empty()
        || !f_clord.read().is_empty()
        || !f_detail.read().is_empty();

    let indices     = timeline_indices.read();
    let total_count = indices.len();
    let display_end = (*display_limit.read()).min(total_count);
    let has_more    = display_end < total_count;

    rsx! {
        div { class: "panel-timeline",
            div { class: "panel-header",
                div { class: "panel-title",
                    if let Some((count, us)) = *parse_stats.read() {
                        span { class: "parse-stats", "Parsed {count} messages in {format_duration(us)}" }
                    }
                    if has_more {
                        span { class: "filter-count", "showing {display_end} of {total_count}" }
                    } else if has_filter && total_count > 0 {
                        span { class: "filter-count", "{total_count} matched" }
                    }
                }
                div { class: "header-actions",
                    if pro && total_count > 0 {
                        button {
                            class: "btn-export-csv",
                            onclick: move |_| {
                                let msgs_snap: Vec<FixMessage> = {
                                    let msgs = messages.read();
                                    timeline_indices.read().iter().map(|&i| msgs[i].clone()).collect()
                                };
                                spawn(async move {
                                    let tag = now_tag();
                                    if let Some(file) = rfd::AsyncFileDialog::new()
                                        .set_file_name(&format!("timeline_{tag}.csv"))
                                        .add_filter("CSV", &["csv"])
                                        .save_file()
                                        .await
                                    {
                                        let csv = messages_to_csv(&msgs_snap);
                                        let _ = std::fs::write(file.path(), csv.as_bytes());
                                    }
                                });
                            },
                            "Export CSV"
                        }
                    }
                    if has_filter {
                        button {
                            class: "btn-clear-filter",
                            onclick: move |_| {
                                f_time.set(String::new());
                                f_time_op.set(String::from("="));
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
                div { class: "tbl-header tbl-timeline-row",
                    span { "Time" }
                    span { "Sender" }
                    span { "Target" }
                    span { "Message" }
                    span { "ID" }
                    span { "Detail" }
                }
                div { class: "tbl-filter tbl-timeline-row",
                    div { class: "time-filter-wrap",
                        select {
                            class: "time-op-select",
                            onchange: move |e| f_time_op.set(e.value()),
                            option { value: "=",  selected: ft_op == "=",  "=" }
                            option { value: ">=", selected: ft_op == ">=", "≥" }
                            option { value: "<=", selected: ft_op == "<=", "≤" }
                        }
                        input { class: "col-filter", placeholder: "2024-01-02 08:00:00.000",
                            value: "{f_time.read()}",
                            oninput: move |e| f_time.set(e.value()),
                        }
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
                div { class: "tbl-body", id: "timeline-scroll",
                    for idx in indices[..display_end].iter().copied() {
                        div {
                            class: if sel == Some(idx) { "tbl-row tbl-timeline-row row-selected" } else { "tbl-row tbl-timeline-row" },
                            onclick: move |_| selected_idx.set(Some(idx)),
                            span { class: "cell-time", "{msgs[idx].time}" }
                            span { "{msgs[idx].sender}" }
                            span { "{msgs[idx].target}" }
                            span { span { class: "badge {badge_class(&msgs[idx].msg_type_raw)}", "{msgs[idx].msg_type_label}" } }
                            span {
                                if !msgs[idx].cl_ord_id.is_empty() {
                                    span { class: "id-clordid", "{msgs[idx].cl_ord_id}" }
                                }
                                if !msgs[idx].quote_id.is_empty() {
                                    span { class: "id-label", "Q:" }
                                    span { class: "id-quoteid", "{msgs[idx].quote_id}" }
                                }
                                if !msgs[idx].quote_req_id.is_empty() {
                                    span { class: "id-label", "QR:" }
                                    span { class: "id-quotereqid", "{msgs[idx].quote_req_id}" }
                                }
                            }
                            span { class: "cell-detail", "{build_detail_text(&msgs[idx])}" }
                        }
                    }
                    if indices.is_empty() {
                        div { class: "empty-state",
                            if has_filter { "No messages match the current filters." }
                            else { "No messages parsed yet." }
                        }
                    }
                    if has_more {
                        div { class: "cap-notice", "Scroll down to load more…" }
                    }
                }
            }
        }
    }
}
