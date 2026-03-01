use dioxus::prelude::*;
use dioxus::document::eval;

use crate::dictionary::badge_class;
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

/// Renders the Timeline panel (left side).
#[component]
pub fn timeline_panel(
    messages: Signal<Vec<FixMessage>>,
    selected_idx: Signal<Option<usize>>,
    skip_heartbeats: Signal<bool>,
    parse_stats: Signal<Option<(usize, u64)>>,
) -> Element {
    let mut f_time   = use_signal(String::new);
    let mut f_sender = use_signal(String::new);
    let mut f_target = use_signal(String::new);
    let mut f_msg    = use_signal(String::new);
    let mut f_clord  = use_signal(String::new);
    let mut f_detail = use_signal(String::new);

    let mut display_limit = use_signal(|| INITIAL_DISPLAY);

    // ── One-time JS scroll listener ──────────────────────────────────────────
    // Reads NO reactive signals → runs exactly once on mount.
    // Installs a native browser scroll listener on #timeline-scroll.
    // When within 200 px of the bottom AND .cap-notice sentinel is present,
    // JS calls dioxus.send(true). Rust increments display_limit.
    use_effect(move || {
        let mut dl = display_limit.clone();
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

    // ── Reset on filter / dataset change ────────────────────────────────────
    // Reads filter signals + message count → re-runs whenever they change.
    use_effect(move || {
        f_time.read();
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

    // ── Memoized filter ──────────────────────────────────────────────────────
    // Re-computes ONLY when a filter signal, skip_heartbeats, or messages change.
    // Crucially, scrolling (display_limit changes) does NOT trigger a re-scan —
    // the cached Vec<usize> is reused, making scroll completely free.
    let timeline_indices = use_memo(move || -> Vec<usize> {
        let skip_hb = *skip_heartbeats.read();
        let msgs    = messages.read();
        let ft_raw  = f_time.read().clone();
        let fs      = f_sender.read().to_ascii_lowercase();
        let fta     = f_target.read().to_ascii_lowercase();
        let fm      = f_msg.read().to_ascii_lowercase();
        let fc      = f_clord.read().to_ascii_lowercase();
        let fd      = f_detail.read().to_ascii_lowercase();

        msgs.iter()
            .enumerate()
            .filter(|(_, m)| !(skip_hb && (m.msg_type_raw == "0" || m.msg_type_raw == "A")))
            .filter(|(_, m)| {
                time_match(&m.time, &ft_raw)
                    && col_match(&m.sender, &fs)
                    && col_match(&m.target, &fta)
                    && col_match(m.msg_type_label, &fm)
                    && col_match(&m.cl_ord_id, &fc)
                    && (fd.is_empty() || col_match(&build_detail_text(m), &fd))
            })
            .map(|(i, _)| i)
            .collect()
    });

    // ── Render-time state ────────────────────────────────────────────────────
    let msgs = messages.read();
    let sel  = *selected_idx.read();

    // Filter strings re-read here for input value bindings and has_filter check.
    let ft_raw = f_time.read().clone();
    let fs  = f_sender.read().to_ascii_lowercase();
    let fta = f_target.read().to_ascii_lowercase();
    let fm  = f_msg.read().to_ascii_lowercase();
    let fc  = f_clord.read().to_ascii_lowercase();
    let fd  = f_detail.read().to_ascii_lowercase();

    let has_filter = !ft_raw.is_empty() || !fs.is_empty() || !fta.is_empty()
        || !fm.is_empty() || !fc.is_empty() || !fd.is_empty();

    // Cached result from the memo — free if filters/messages haven't changed.
    let indices     = timeline_indices.read();
    let total_count = indices.len();
    let display_end = (*display_limit.read()).min(total_count);
    let has_more    = display_end < total_count;

    rsx! {
        div { class: "panel-timeline",
            div { class: "panel-header",
                div { class: "panel-title",
                    h2 { "Timeline" }
                    if let Some((count, us)) = *parse_stats.read() {
                        span { class: "parse-stats", "parsed {count} messages in {format_duration(us)}" }
                    }
                    if has_more {
                        span { class: "filter-count", "showing {display_end} of {total_count}" }
                    } else if has_filter && total_count > 0 {
                        span { class: "filter-count", "{total_count} matched" }
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
                div { class: "tbl-header tbl-timeline-row",
                    span { "Time" }
                    span { "Sender" }
                    span { "Target" }
                    span { "Message" }
                    span { "Client order ID" }
                    span { "Detail" }
                }
                div { class: "tbl-filter tbl-timeline-row",
                    input { class: "col-filter", placeholder: "exact · >= · <=",
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
                div { class: "tbl-body", id: "timeline-scroll",
                    for idx in indices[..display_end].iter().copied() {
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
                    if indices.is_empty() {
                        div { class: "empty-state",
                            if has_filter { "No messages match the current filters." }
                            else { "No messages parsed yet." }
                        }
                    }
                    // Sentinel: JS checks for this element before sending load-more.
                    if has_more {
                        div { class: "cap-notice", "Scroll down to load more…" }
                    }
                }
            }
        }
    }
}
