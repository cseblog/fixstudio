use std::collections::HashSet;

use dioxus::prelude::*;
use dioxus::document::eval;

use crate::dictionary::badge_class;
use crate::export::{messages_to_csv, now_tag};
use crate::model::FixMessage;
use crate::tab::message_key;

const LOAD_MORE: usize = 1000;
const INITIAL_DISPLAY: usize = 1000;

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
    // Per-tab filter + view state. Lives on Tab so switching tabs preserves
    // each tab's filters / scroll cap / open-or-closed filter row.
    mut f_time:        Signal<String>,
    mut f_time_op:     Signal<String>,
    mut f_sender:      Signal<String>,
    mut f_target:      Signal<String>,
    mut f_msg:         Signal<String>,
    mut f_clord:       Signal<String>,
    mut f_detail:      Signal<String>,
    mut display_limit: Signal<usize>,
    mut filters_open:  Signal<bool>,
    #[props(default)]
    compare_keys: Option<ReadSignal<HashSet<String>>>,
) -> Element {

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
            div { class: "stats-strip",
                if let Some((count, us)) = *parse_stats.read() {
                    span { class: "stat-pill stat-pill-good",
                        span { class: "stat-pill-num", "{count}" }
                        span { class: "stat-pill-unit", "msgs" }
                        span { class: "stat-pill-sep", "·" }
                        span { class: "stat-pill-num", "{format_duration(us)}" }
                    }
                }
                if has_more {
                    span { class: "stat-pill stat-pill-meta",
                        span { class: "stat-pill-num", "{display_end}" }
                        span { class: "stat-pill-unit", "of" }
                        span { class: "stat-pill-num", "{total_count}" }
                    }
                } else if has_filter && total_count > 0 {
                    span { class: "stat-pill stat-pill-meta",
                        span { class: "stat-pill-num", "{total_count}" }
                        span { class: "stat-pill-unit", "matched" }
                    }
                }
                div { class: "stats-spacer" }
                button {
                    class: if *filters_open.read() || has_filter { "btn-icon btn-icon-on" } else { "btn-icon" },
                    title: "Toggle column filters",
                    onclick: move |_| { let v = !*filters_open.read(); filters_open.set(v); },
                    "⏷ Filters"
                }
                label { class: "check-label",
                    input {
                        r#type: "checkbox",
                        checked: *skip_heartbeats.read(),
                        onchange: move |evt: Event<FormData>| skip_heartbeats.set(evt.checked()),
                    }
                    " Skip heartbeats"
                }
                if total_count > 0 {
                    button {
                        class: "btn-icon",
                        title: "Export filtered rows to CSV",
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
                        "⬇ CSV"
                    }
                }
                if has_filter {
                    button {
                        class: "btn-icon btn-icon-clear",
                        title: "Clear all filters",
                        onclick: move |_| {
                            f_time.set(String::new());
                            f_time_op.set(String::from("="));
                            f_sender.set(String::new());
                            f_target.set(String::new());
                            f_msg.set(String::new());
                            f_clord.set(String::new());
                            f_detail.set(String::new());
                        },
                        "✕"
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
                if *filters_open.read() || has_filter {
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
                }
                div { class: "tbl-body", id: "timeline-scroll",
                    {
                        // Pre-compute pairs (idx, prev_idx_in_list) for date dedup + ClOrdID grouping rail.
                        // Guard against stale `timeline_indices` memo whose entries can briefly
                        // exceed `msgs.len()` right after a replace.
                        let msg_count = msgs.len();
                        let rendered: Vec<(usize, Option<usize>)> = indices[..display_end]
                            .iter()
                            .copied()
                            .filter(|&idx| idx < msg_count)
                            .enumerate()
                            .map(|(i, idx)| {
                                let prev = if i == 0 {
                                    None
                                } else {
                                    indices.get(i - 1).copied().filter(|&p| p < msg_count)
                                };
                                (idx, prev)
                            })
                            .collect();
                        rsx! {
                            for (idx, prev_idx) in rendered.into_iter() {
                                {
                                    let m = &msgs[idx];
                                    // Date dedup: hide leading date when same as previous row's date.
                                    let prev_date = prev_idx.map(|p| &msgs[p].time[..msgs[p].time.len().min(10)]);
                                    let cur_date  = &m.time[..m.time.len().min(10)];
                                    let cur_time_only = if m.time.len() > 11 { &m.time[11..] } else { &m.time[..] };
                                    let show_date = prev_date.map(|d| d != cur_date).unwrap_or(true);
                                    // ClOrdID grouping: rows share a colored left rail when ClOrdID matches the previous row.
                                    let same_group = prev_idx
                                        .map(|p| !m.cl_ord_id.is_empty() && msgs[p].cl_ord_id == m.cl_ord_id)
                                        .unwrap_or(false);
                                    let group_color = if !m.cl_ord_id.is_empty() {
                                        // Cheap, stable hash → palette of 6 muted hues.
                                        let h = m.cl_ord_id.bytes().fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
                                        match h % 6 {
                                            0 => "#2f6b2f", 1 => "#15467a", 2 => "#b78427",
                                            3 => "#7a3a8a", 4 => "#4a7a4a", _ => "#b22222",
                                        }
                                    } else { "transparent" };
                                    let mut cls = String::from("tbl-row tbl-timeline-row");
                                    if Some(idx) == sel { cls.push_str(" row-selected"); }
                                    if same_group { cls.push_str(" row-group-cont"); }
                                    if let Some(ck) = compare_keys.as_ref() {
                                        if let Some(k) = message_key(m) {
                                            if ck.read().contains(&k) { cls.push_str(" row-match"); }
                                            else                       { cls.push_str(" row-diverge"); }
                                        }
                                    }
                                    rsx! {
                                        div {
                                            class: "{cls}",
                                            style: "box-shadow: inset 3px 0 0 0 {group_color};",
                                            onclick: move |_| selected_idx.set(Some(idx)),
                                            span { class: "cell-time",
                                                if show_date {
                                                    span { class: "cell-time-date", "{cur_date} " }
                                                }
                                                span { class: "cell-time-clock", "{cur_time_only}" }
                                            }
                                            span { "{m.sender}" }
                                            span { "{m.target}" }
                                            span { span { class: "badge {badge_class(&m.msg_type_raw)}", "{m.msg_type_label}" } }
                                            span {
                                                if !m.cl_ord_id.is_empty() {
                                                    span { class: "id-clordid", "{m.cl_ord_id}" }
                                                }
                                                if !m.quote_id.is_empty() {
                                                    span { class: "id-label", "Q:" }
                                                    span { class: "id-quoteid", "{m.quote_id}" }
                                                }
                                                if !m.quote_req_id.is_empty() {
                                                    span { class: "id-label", "QR:" }
                                                    span { class: "id-quotereqid", "{m.quote_req_id}" }
                                                }
                                            }
                                            span { class: "cell-detail", "{build_detail_text(m)}" }
                                        }
                                    }
                                }
                            }
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
