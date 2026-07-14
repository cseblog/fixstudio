use std::collections::HashSet;

use dioxus::prelude::*;
use dioxus::document::eval;

use crate::dictionary::badge_class;
use crate::export::{messages_to_csv, now_tag};
use crate::model::FixMessage;
use crate::tab::message_key;

const LOAD_MORE: usize = 1000;
const INITIAL_DISPLAY: usize = 1000;

// ── Chain-aware ID filter index ──────────────────────────────────────────────
//
// Maps every (ClOrdID, QuoteID, QuoteReqID) string to the same chain root
// when those ids appear together on any single message. A user-typed substring
// resolves to a set of chain roots; every message in those chains is included
// in the filter, even if its own ids don't textually match the query.

/// Per-message chain root: `msg_root[i]` is the union-find root shared by every
/// message in message `i`'s trade chain, or `u32::MAX` when it has no chain id.
#[derive(PartialEq)]
pub struct ChainIndex {
    pub msg_root: Vec<u32>,
}

/// Path-compressing union-find. Asserts that x is a valid index.
fn uf_find(parent: &mut [u32], x: u32) -> u32 {
    debug_assert!((x as usize) < parent.len());
    let mut r = x;
    while parent[r as usize] != r { r = parent[r as usize]; }
    // One-pass path compression
    let mut c = x;
    while parent[c as usize] != r {
        let nxt = parent[c as usize];
        parent[c as usize] = r;
        c = nxt;
    }
    r
}

/// An id value is usable for chain linking only if it looks like a real,
/// unique identifier. Wildcard / mass-action placeholders (e.g. QuoteID
/// "* - ALL" on a QuoteCancel that cancels every quote) must be excluded —
/// otherwise union-find fuses every unrelated RFQ that carries the placeholder
/// into one giant chain, and filtering by any single id drags in the whole log.
fn is_linkable_id(v: &str) -> bool {
    !v.is_empty() && !v.contains('*')
}

/// Build the chain index over the given message set. Pure function; tested
/// in isolation via the unit tests at the bottom of this file.
pub fn build_chain_index(msgs: &[FixMessage]) -> ChainIndex {
    let n = msgs.len();
    let mut id_to_chain: ahash::AHashMap<String, u32> = ahash::AHashMap::default();
    let mut parent:   Vec<u32> = Vec::with_capacity(n / 2);
    let mut msg_root: Vec<u32> = vec![u32::MAX; n];

    // Tags pulled from the raw message fields in addition to the three typed
    // slots — OrigClOrdID (41) is critical because OrderCancelRequest /
    // OrderCancelReplaceRequest / QuoteCancel reference the original chain
    // through tag 41 rather than re-stating ClOrdID / QuoteID / QuoteReqID.
    const EXTRA_LINK_TAGS: [u16; 1] = [41];

    let mut ids_buf: Vec<String> = Vec::with_capacity(6);
    for (i, m) in msgs.iter().enumerate() {
        ids_buf.clear();
        if is_linkable_id(m.cl_ord_id.as_str())    { ids_buf.push(m.cl_ord_id.as_str().to_ascii_lowercase()); }
        if is_linkable_id(m.quote_id.as_str())     { ids_buf.push(m.quote_id.as_str().to_ascii_lowercase()); }
        if is_linkable_id(m.quote_req_id.as_str()) { ids_buf.push(m.quote_req_id.as_str().to_ascii_lowercase()); }
        for f in &m.fields {
            if EXTRA_LINK_TAGS.contains(&f.tag) {
                let v = f.value_in(&m.arena);
                if is_linkable_id(v) { ids_buf.push(v.to_ascii_lowercase()); }
            }
        }
        if ids_buf.is_empty() { continue; }

        // Unite any already-known chains shared by this message's ids.
        let mut anchor: Option<u32> = None;
        for id in ids_buf.iter() {
            if let Some(&c) = id_to_chain.get(id) {
                let rc = uf_find(&mut parent, c);
                anchor = Some(match anchor {
                    None    => rc,
                    Some(a) => {
                        let ra = uf_find(&mut parent, a);
                        if ra != rc { parent[rc as usize] = ra; }
                        ra
                    }
                });
            }
        }
        let chain_id = match anchor {
            Some(c) => c,
            None => {
                let c = parent.len() as u32;
                parent.push(c);
                c
            }
        };
        for id in ids_buf.iter() {
            id_to_chain.entry(id.clone()).or_insert(chain_id);
        }
        msg_root[i] = chain_id;
    }
    // Resolve every chain reference to its final root.
    for v in msg_root.iter_mut() {
        if *v != u32::MAX { *v = uf_find(&mut parent, *v); }
    }
    ChainIndex { msg_root }
}

// ── Timeline filter (pure, tested) ───────────────────────────────────────────
//
// All filter state needed by `apply_timeline_filter` in raw form (as typed by
// the user). Keeping it as plain `String`s + a bool lets the function take a
// single borrow and lets us write tests without touching any Dioxus signal.

#[derive(Debug, Default, Clone)]
pub struct TimelineFilter {
    pub skip_heartbeats: bool,
    /// User-typed time value (may be empty).
    pub f_time:    String,
    /// Time comparison operator: "=", ">=", or "<=".
    pub f_time_op: String,
    pub f_sender:  String,
    pub f_target:  String,
    pub f_msg:     String,
    /// "ID column" filter — matches ClOrdID, OrigClOrdID, QuoteID, QuoteReqID
    /// directly, then expands via chain_index for transitive chain membership.
    pub f_clord:   String,
    pub f_detail:  String,
}

/// Pure timeline-filter pipeline. Returns indices into `msgs` for rows that
/// pass every column filter, sorted newest-first by `time`. Identical to the
/// inline closure in `timeline_panel`'s use_memo, just extracted so the test
/// suite can exercise every code path without spinning up Dioxus.
pub fn apply_timeline_filter(
    msgs: &[FixMessage],
    chain: &ChainIndex,
    f:    &TimelineFilter,
) -> Vec<usize> {
    // Combine operator + value into the format `time_match` expects:
    //   "="  → plain substring match; ">=" / "<=" → prefixed range query.
    let ft_raw = if f.f_time_op == "=" || f.f_time.is_empty() {
        f.f_time.clone()
    } else {
        format!("{}{}", f.f_time_op, f.f_time)
    };
    // Trim then lower-case every filter — pasted IDs from terminals or
    // spreadsheets often carry leading / trailing whitespace that would
    // otherwise silently break substring matches against the trimmed FIX
    // values stored on the message.
    let fs = f.f_sender.trim().to_ascii_lowercase();
    let fta = f.f_target.trim().to_ascii_lowercase();
    let fm = f.f_msg.trim().to_ascii_lowercase();
    let fc = f.f_clord.trim().to_ascii_lowercase();
    let fd = f.f_detail.trim().to_ascii_lowercase();

    // Chain-aware ID filter, EXACT-seeded. Seed roots = the chain roots of every
    // message whose OWN id field exactly equals the typed value. A row then
    // passes if it exact-matches directly OR shares a seed root. Exact (not
    // substring) matching means "20cf0278" never drags in "20cf02781", and
    // wildcard placeholders are already stripped from chain links, so a single
    // id keys exactly its own trade — request, quote, order, and ER — nothing else.
    let seed_roots: ahash::AHashSet<u32> = if fc.is_empty() {
        ahash::AHashSet::default()
    } else {
        msgs.iter().enumerate()
            .filter(|(_, m)| id_exact_match(m, &fc))
            .filter_map(|(i, _)| {
                let r = chain.msg_root[i];
                (r != u32::MAX).then_some(r)
            })
            .collect()
    };

    let mut indices: Vec<usize> = msgs.iter()
        .enumerate()
        .filter(|(_, m)| !(f.skip_heartbeats && (m.msg_type_raw == "0" || m.msg_type_raw == "A")))
        .filter(|(i, m)| {
            if !fc.is_empty() {
                let root = chain.msg_root[*i];
                let pass = id_exact_match(m, &fc)
                    || (root != u32::MAX && seed_roots.contains(&root));
                if !pass { return false; }
            }
            time_match(&m.time, &ft_raw)
                && col_match(&m.sender, &fs)
                && col_match(&m.target, &fta)
                && col_match(m.msg_type_label, &fm)
                && (fd.is_empty() || col_match(&build_detail_text(m), &fd))
        })
        .map(|(i, _)| i)
        .collect();
    indices.sort_unstable_by(|&a, &b| msgs[b].time.cmp(&msgs[a].time));
    indices
}

/// True when the typed value `fc` (already trimmed + ASCII-lowercased) EXACTLY
/// equals one of message `m`'s identifier fields: ClOrdID, QuoteID, QuoteReqID,
/// or OrigClOrdID (tag 41). Case-insensitive; no substring matching, so a
/// specific id never matches a longer id that merely starts with it.
fn id_exact_match(m: &FixMessage, fc: &str) -> bool {
    if m.cl_ord_id.eq_ignore_ascii_case(fc)
        || m.quote_id.eq_ignore_ascii_case(fc)
        || m.quote_req_id.eq_ignore_ascii_case(fc)
    {
        return true;
    }
    m.fields.iter()
        .any(|f| f.tag == 41 && f.value_in(&m.arena).eq_ignore_ascii_case(fc))
}

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
    /// "Jump to latency chain": fires with the chosen id when the user
    /// clicks the ↗ action on an ID line. Host wires it to switch view
    /// mode to Lifecycle and pre-fill the chain filter.
    on_jump_to_chain: EventHandler<String>,
    /// Indices of messages that failed validation. Empty set means
    /// either no errors OR the host capped validation for large logs.
    #[props(default)]
    invalid_indices: Option<ReadSignal<HashSet<usize>>>,
    /// Fires with a message index when the user clicks the validator
    /// gutter on an invalid row. Host wires it to switch to Validator.
    #[props(default)]
    on_jump_to_validator: EventHandler<usize>,
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

    // Per-message chain root — built once per messages change via union-find
    // over (ClOrdID, QuoteID, QuoteReqID). Lets the ID column filter be
    // CHAIN-AWARE: typing an RFQ also matches the Quote, the linked NOS, the
    // ExecutionReports, and any cancels — every message in the same trade
    // chain. Cost: O(n α(n)) build, then O(unique_ids) per filter input change.
    let chain_index = use_memo(move || -> std::sync::Arc<ChainIndex> {
        std::sync::Arc::new(build_chain_index(&messages.read()))
    });

    // Memoized: re-computes only when filters, skip_heartbeats, or messages change.
    // Scrolling (display_limit changes) does NOT trigger a re-scan.
    let timeline_indices = use_memo(move || -> Vec<usize> {
        let msgs  = messages.read();
        let chain = chain_index.read();
        let f = TimelineFilter {
            skip_heartbeats: *skip_heartbeats.read(),
            f_time:    f_time.read().clone(),
            f_time_op: f_time_op.read().clone(),
            f_sender:  f_sender.read().clone(),
            f_target:  f_target.read().clone(),
            f_msg:     f_msg.read().clone(),
            f_clord:   f_clord.read().clone(),
            f_detail:  f_detail.read().clone(),
        };
        apply_timeline_filter(&msgs, &chain, &f)
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
                                    let row_invalid = invalid_indices.as_ref()
                                        .map(|s| s.read().contains(&idx))
                                        .unwrap_or(false);
                                    if row_invalid { cls.push_str(" row-invalid"); }
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
                                            {
                                                // Vertical stack of every chain-relevant id on this
                                                // message — ClOrdID (C), QuoteID (Q), QuoteReqID (QR)
                                                // and OrigClOrdID (O, tag 41 — used by cancels). The
                                                // ID-column filter substring-matches against ALL of
                                                // them so typing any one id reveals the row.
                                                let orig_cl_ord_id: Option<&str> = m.fields.iter()
                                                    .find(|f| f.tag == 41)
                                                    .map(|f| f.value_in(&m.arena))
                                                    .filter(|v| !v.is_empty());
                                                {
                                                    // Capture owned ids upfront so each row's jump
                                                    // button closure can `move` its own copy.
                                                    let id_c  = m.cl_ord_id.as_str().to_string();
                                                    let id_o  = orig_cl_ord_id.map(|s| s.to_string());
                                                    let id_q  = m.quote_id.as_str().to_string();
                                                    let id_qr = m.quote_req_id.as_str().to_string();
                                                    rsx! {
                                                        span { class: "cell-id-stack",
                                                            if !id_c.is_empty() {
                                                                span { class: "id-line",
                                                                    span { class: "id-label", "C:" }
                                                                    span { class: "id-clordid", "{id_c}" }
                                                                    {
                                                                        let v = id_c.clone();
                                                                        rsx! {
                                                                            button {
                                                                                class: "id-jump",
                                                                                title: "Jump to latency chain for {v}",
                                                                                onclick: move |e: MouseEvent| {
                                                                                    e.stop_propagation();
                                                                                    on_jump_to_chain.call(v.clone());
                                                                                },
                                                                                "↗"
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            if let Some(oc) = id_o {
                                                                span { class: "id-line",
                                                                    span { class: "id-label", "O:" }
                                                                    span { class: "id-orig", "{oc}" }
                                                                    {
                                                                        let v = oc.clone();
                                                                        rsx! {
                                                                            button {
                                                                                class: "id-jump",
                                                                                title: "Jump to latency chain for {v}",
                                                                                onclick: move |e: MouseEvent| {
                                                                                    e.stop_propagation();
                                                                                    on_jump_to_chain.call(v.clone());
                                                                                },
                                                                                "↗"
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            if !id_q.is_empty() {
                                                                span { class: "id-line",
                                                                    span { class: "id-label", "Q:" }
                                                                    span { class: "id-quoteid", "{id_q}" }
                                                                    {
                                                                        let v = id_q.clone();
                                                                        rsx! {
                                                                            button {
                                                                                class: "id-jump",
                                                                                title: "Jump to latency chain for {v}",
                                                                                onclick: move |e: MouseEvent| {
                                                                                    e.stop_propagation();
                                                                                    on_jump_to_chain.call(v.clone());
                                                                                },
                                                                                "↗"
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            if !id_qr.is_empty() {
                                                                span { class: "id-line",
                                                                    span { class: "id-label", "QR:" }
                                                                    span { class: "id-quotereqid", "{id_qr}" }
                                                                    {
                                                                        let v = id_qr.clone();
                                                                        rsx! {
                                                                            button {
                                                                                class: "id-jump",
                                                                                title: "Jump to latency chain for {v}",
                                                                                onclick: move |e: MouseEvent| {
                                                                                    e.stop_propagation();
                                                                                    on_jump_to_chain.call(v.clone());
                                                                                },
                                                                                "↗"
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            span { class: "cell-detail", "{build_detail_text(m)}" }
                                            if row_invalid {
                                                button {
                                                    class: "row-invalid-mark",
                                                    title: "Validation errors — click to open in Validator",
                                                    onclick: move |e: MouseEvent| {
                                                        e.stop_propagation();
                                                        selected_idx.set(Some(idx));
                                                        on_jump_to_validator.call(idx);
                                                    },
                                                    "⚠"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if indices.is_empty() {
                        div { class: "empty-state",
                            if has_filter {
                                span { class: "empty-state-icon",  "🔍" }
                                span { class: "empty-state-title", "No matching messages" }
                                span { class: "empty-state-hint",
                                    "No row passes the current column filters. Clear or relax a filter to see more."
                                }
                            } else {
                                span { class: "empty-state-icon",  "📭" }
                                span { class: "empty-state-title", "No messages yet" }
                                span { class: "empty-state-hint",
                                    "Paste FIX text above, load a file, or pick a sample to get started."
                                }
                            }
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


#[cfg(test)]
mod chain_tests {
    use super::*;
    use crate::parser::parse_all;

    /// Synthetic RFQ chain: QuoteRequest → Quote → NewOrderSingle → ER FILL.
    /// Tags: 131=QR-1 (RFQ id), 117=QT-9 (QuoteID), 11=CO-7 (ClOrdID).
    /// - QuoteRequest carries 131 only.
    /// - Quote carries 131 + 117 (links RFQ → QuoteID).
    /// - NewOrderSingle carries 117 + 11 (links QuoteID → ClOrdID).
    /// - ExecutionReport carries 11.
    /// A user typing "QR-1" must surface all four.
    fn synth_chain() -> Vec<FixMessage> {
        let raw = concat!(
            "8=FIX.4.4|9=1|35=R|49=A|56=B|34=1|52=20240101-09:00:00.000|131=QR-1|55=AAPL|10=000|",
            "8=FIX.4.4|9=1|35=S|49=B|56=A|34=2|52=20240101-09:00:00.050|131=QR-1|117=QT-9|55=AAPL|10=000|",
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=3|52=20240101-09:00:00.100|117=QT-9|11=CO-7|55=AAPL|54=1|38=100|40=2|10=000|",
            "8=FIX.4.4|9=1|35=8|49=B|56=A|34=4|52=20240101-09:00:00.150|11=CO-7|150=F|39=2|10=000|",
        );
        parse_all(raw)
    }

    #[test]
    fn rfq_id_resolves_to_full_chain() {
        let msgs = synth_chain();
        let idx  = build_chain_index(&msgs);
        // All four messages should share the same chain root.
        let roots: Vec<u32> = idx.msg_root.iter().copied().collect();
        assert_eq!(roots.len(), 4);
        assert_ne!(roots[0], u32::MAX, "QuoteRequest must have a chain");
        assert!(roots.iter().all(|&r| r == roots[0]),
            "all messages in the same chain must share a root, got {:?}", roots);
    }

    #[test]
    fn wildcard_ids_do_not_link_chains() {
        // A shared wildcard QuoteID "* - ALL" must NOT fuse two otherwise
        // unrelated RFQs into one chain root.
        let raw = concat!(
            "8=FIX.4.4|9=1|35=R|49=A|56=B|34=1|52=20240101-09:00:00.000|131=RFQ-A|117=* - ALL|55=AAPL|10=000|",
            "8=FIX.4.4|9=1|35=R|49=A|56=B|34=2|52=20240101-09:00:00.001|131=RFQ-B|117=* - ALL|55=MSFT|10=000|",
        );
        let msgs = parse_all(raw);
        let idx  = build_chain_index(&msgs);
        assert_ne!(idx.msg_root[0], idx.msg_root[1],
            "wildcard QuoteID must not link two unrelated RFQs");
    }

    #[test]
    fn unrelated_messages_have_distinct_chains() {
        let raw = concat!(
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=1|52=20240101-09:00:00.000|11=ORD-A|55=AAPL|10=000|",
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=2|52=20240101-09:00:00.001|11=ORD-B|55=MSFT|10=000|",
        );
        let msgs = parse_all(raw);
        let idx  = build_chain_index(&msgs);
        assert_ne!(idx.msg_root[0], idx.msg_root[1],
            "two unrelated NOS messages must NOT share a chain root");
    }

    #[test]
    fn messages_without_chain_ids_have_max_root() {
        let raw = "8=FIX.4.4|9=1|35=0|49=A|56=B|34=1|52=20240101-09:00:00.000|10=000|";
        let msgs = parse_all(raw);
        let idx  = build_chain_index(&msgs);
        assert_eq!(idx.msg_root[0], u32::MAX,
            "heartbeat with no chain ids should be sentinel u32::MAX");
    }
}

#[cfg(test)]
mod filter_tests {
    use super::*;
    use crate::parser::parse_all;

    /// 4-msg trade chain: QuoteRequest → Quote → NOS → ER FILL.
    /// Chain ids: RFQ=QR-1, QuoteID=QT-9, ClOrdID=CO-7.
    fn synth_chain() -> Vec<FixMessage> {
        let raw = concat!(
            "8=FIX.4.4|9=1|35=R|49=A|56=B|34=1|52=20240101-09:00:00.000|131=QR-1|55=AAPL|10=000|",
            "8=FIX.4.4|9=1|35=S|49=B|56=A|34=2|52=20240101-09:00:00.050|131=QR-1|117=QT-9|55=AAPL|10=000|",
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=3|52=20240101-09:00:00.100|117=QT-9|11=CO-7|55=AAPL|54=1|38=100|40=2|10=000|",
            "8=FIX.4.4|9=1|35=8|49=B|56=A|34=4|52=20240101-09:00:00.150|11=CO-7|150=F|39=2|10=000|",
        );
        parse_all(raw)
    }

    /// Three independent orders, NO chain links between them.
    fn synth_three_orders() -> Vec<FixMessage> {
        let raw = concat!(
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=1|52=20240101-09:00:00.000|11=ORD-1|55=AAPL|10=000|",
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=2|52=20240101-09:00:00.001|11=ORD-2|55=MSFT|10=000|",
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=3|52=20240101-09:00:00.002|11=ORD-3|55=AMZN|10=000|",
        );
        parse_all(raw)
    }

    /// Order + Cancel pair where the Cancel uses tag 41 (OrigClOrdID) to point
    /// at the original NOS. The shared OrigClOrdID is the only chain link.
    fn synth_order_with_cancel() -> Vec<FixMessage> {
        let raw = concat!(
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=1|52=20240101-09:00:00.000|11=ORD-A|55=AAPL|10=000|",
            "8=FIX.4.4|9=1|35=F|49=A|56=B|34=2|52=20240101-09:00:00.010|11=CXL-A|41=ORD-A|55=AAPL|10=000|",
        );
        parse_all(raw)
    }

    fn empty_filter() -> TimelineFilter {
        TimelineFilter { f_time_op: "=".into(), ..Default::default() }
    }

    /// Mirrors real RFS-gateway data (ing-rfq-gateway): every message of one
    /// RFQ shares the same QuoteReqID (131). QuoteRequests carry a WILDCARD
    /// QuoteID "* - ALL" (117) which must NOT be treated as a chain link, or
    /// union-find fuses every unrelated RFQ into one giant chain. Three
    /// independent RFQs: 20cf028a / 20cf028b / 20cf028c.
    fn synth_rfs_wildcard() -> Vec<FixMessage> {
        let raw = concat!(
            "8=FIX.4.4|35=R|49=MM|56=LD4|34=1|52=20240101-09:00:00.000|131=20cf028a|117=* - ALL|55=EUR/USD|10=000|",
            "8=FIX.4.4|35=S|49=LD4|56=MM|34=2|52=20240101-09:00:00.010|131=20cf028a|117=20cf028a1|55=EUR/USD|10=000|",
            "8=FIX.4.4|35=Z|49=MM|56=LD4|34=3|52=20240101-09:00:00.020|131=20cf028a|117=20cf028a1|55=EUR/USD|10=000|",
            "8=FIX.4.4|35=R|49=MM|56=LD4|34=4|52=20240101-09:00:01.000|131=20cf028b|117=* - ALL|55=USD/HUF|10=000|",
            "8=FIX.4.4|35=S|49=LD4|56=MM|34=5|52=20240101-09:00:01.010|131=20cf028b|117=20cf028b1|55=USD/HUF|10=000|",
            "8=FIX.4.4|35=Z|49=MM|56=LD4|34=6|52=20240101-09:00:01.020|131=20cf028b|117=20cf028b1|55=USD/HUF|10=000|",
            "8=FIX.4.4|35=R|49=MM|56=LD4|34=7|52=20240101-09:00:02.000|131=20cf028c|117=* - ALL|55=USD/TRY|10=000|",
            "8=FIX.4.4|35=S|49=LD4|56=MM|34=8|52=20240101-09:00:02.010|131=20cf028c|117=20cf028c1|55=USD/TRY|10=000|",
            "8=FIX.4.4|35=Z|49=MM|56=LD4|34=9|52=20240101-09:00:02.020|131=20cf028c|117=20cf028c1|55=USD/TRY|10=000|",
        );
        parse_all(raw)
    }

    #[test]
    fn id_filter_wildcard_quoteid_does_not_merge_rfqs() {
        let msgs = synth_rfs_wildcard();
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        f.f_clord = "20cf028a".into();
        let r = apply_timeline_filter(&msgs, &idx, &f);
        assert_eq!(r.len(), 3,
            "keying one RFQ must return only its 3 messages, not RFQs linked \
             solely by the wildcard QuoteID '* - ALL'");
        for &i in &r {
            assert_eq!(msgs[i].quote_req_id, "20cf028a", "leaked an unrelated RFQ row");
        }
    }

    /// Hierarchical ids where one is a prefix of another (20cf0278 vs 20cf02781,
    /// straight from the screenshot). Exact match must not bleed the longer id.
    fn synth_prefix_ids() -> Vec<FixMessage> {
        let raw = concat!(
            "8=FIX.4.4|35=R|49=MM|56=LD4|34=1|52=20240101-09:00:00.000|131=20cf0278|55=X|10=000|",
            "8=FIX.4.4|35=S|49=LD4|56=MM|34=2|52=20240101-09:00:00.010|131=20cf0278|117=20cf0278q|55=X|10=000|",
            "8=FIX.4.4|35=R|49=MM|56=LD4|34=3|52=20240101-09:00:01.000|131=20cf02781|55=Y|10=000|",
            "8=FIX.4.4|35=S|49=LD4|56=MM|34=4|52=20240101-09:00:01.010|131=20cf02781|117=20cf02781q|55=Y|10=000|",
        );
        parse_all(raw)
    }

    #[test]
    fn id_filter_exact_no_prefix_bleed() {
        let msgs = synth_prefix_ids();
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        f.f_clord = "20cf0278".into();
        let r = apply_timeline_filter(&msgs, &idx, &f);
        assert_eq!(r.len(), 2,
            "exact QR match must surface only 20cf0278, not the longer 20cf02781");
        for &i in &r {
            assert_eq!(msgs[i].quote_req_id, "20cf0278", "prefix bleed into 20cf02781");
        }
    }

    // ── ID filter: direct substring ─────────────────────────────────────

    #[test]
    fn id_filter_matches_cl_ord_id() {
        let msgs = synth_three_orders();
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        f.f_clord = "ORD-2".into();
        let r = apply_timeline_filter(&msgs, &idx, &f);
        assert_eq!(r.len(), 1);
        assert_eq!(msgs[r[0]].cl_ord_id, "ORD-2");
    }

    #[test]
    fn id_filter_matches_quote_req_id() {
        let msgs = synth_chain();
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        // QuoteRequest carries 131=QR-1; should match by RFQ id alone.
        f.f_clord = "QR-1".into();
        let r = apply_timeline_filter(&msgs, &idx, &f);
        // All 4 chain members share the chain, so all should surface.
        assert_eq!(r.len(), 4);
    }

    #[test]
    fn id_filter_matches_quote_id() {
        let msgs = synth_chain();
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        f.f_clord = "QT-9".into();
        let r = apply_timeline_filter(&msgs, &idx, &f);
        assert_eq!(r.len(), 4, "chain expansion should pull RFQ + NOS + ER");
    }

    #[test]
    fn id_filter_matches_orig_cl_ord_id_via_direct() {
        // Cancel message carries 41=ORD-A. Original NOS has 11=ORD-A.
        // Typing ORD-A should surface BOTH rows even without chain expansion.
        let msgs = synth_order_with_cancel();
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        f.f_clord = "ORD-A".into();
        let r = apply_timeline_filter(&msgs, &idx, &f);
        assert_eq!(r.len(), 2, "NOS + Cancel both reference ORD-A");
    }

    #[test]
    fn id_filter_case_insensitive() {
        let msgs = synth_three_orders();
        let idx  = build_chain_index(&msgs);
        for q in ["ord-1", "ORD-1", "Ord-1", "oRd-1"] {
            let mut f = empty_filter();
            f.f_clord = q.into();
            let r = apply_timeline_filter(&msgs, &idx, &f);
            assert_eq!(r.len(), 1, "case-insensitive match failed for {q:?}");
        }
    }

    #[test]
    fn id_filter_trims_whitespace() {
        let msgs = synth_three_orders();
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        f.f_clord = "   ORD-2   ".into();
        let r = apply_timeline_filter(&msgs, &idx, &f);
        assert_eq!(r.len(), 1, "leading/trailing whitespace must be trimmed");
    }

    #[test]
    fn id_filter_is_exact_not_substring() {
        let msgs = synth_three_orders();
        let idx  = build_chain_index(&msgs);
        // A bare prefix must NOT match — exact ids only.
        let mut f = empty_filter();
        f.f_clord = "ORD".into();
        assert!(apply_timeline_filter(&msgs, &idx, &f).is_empty(),
            "'ORD' is not a full id and must match nothing");
        // The full id matches exactly one row.
        f.f_clord = "ORD-1".into();
        let r = apply_timeline_filter(&msgs, &idx, &f);
        assert_eq!(r.len(), 1);
        assert_eq!(msgs[r[0]].cl_ord_id, "ORD-1");
    }

    #[test]
    fn id_filter_no_match_returns_empty() {
        let msgs = synth_three_orders();
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        f.f_clord = "NEVER-EXISTS".into();
        let r = apply_timeline_filter(&msgs, &idx, &f);
        assert!(r.is_empty());
    }

    #[test]
    fn empty_id_filter_returns_all() {
        let msgs = synth_three_orders();
        let idx  = build_chain_index(&msgs);
        let r = apply_timeline_filter(&msgs, &idx, &empty_filter());
        assert_eq!(r.len(), 3);
    }

    // ── ID filter: junk / placeholder ids don't bleed ───────────────────

    #[test]
    fn id_filter_junk_chain_does_not_bleed() {
        // 100 messages — 99 share the placeholder ClOrdID "0" (one giant junk
        // chain) plus 1 distinct "REAL-1". Exact-match keying "REAL-1" must
        // surface exactly that row: it neither substring-collides with "0" nor
        // shares the junk chain's root.
        let mut raw = String::new();
        for i in 0..99 {
            raw.push_str(&format!(
                "8=FIX.4.4|9=1|35=D|49=A|56=B|34={i}|52=20240101-09:00:00.000|11=0|55=AAPL|10=000|",
            ));
        }
        raw.push_str("8=FIX.4.4|9=1|35=D|49=A|56=B|34=999|52=20240101-09:00:00.000|11=REAL-1|55=AAPL|10=000|");
        let msgs = parse_all(&raw);
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        f.f_clord = "REAL-1".into();
        let r = apply_timeline_filter(&msgs, &idx, &f);
        assert_eq!(r.len(), 1, "only the REAL-1 row should match");
    }

    #[test]
    fn id_filter_seeds_full_chain_from_exact_id() {
        // Exact keying of the RFQ id seeds the chain root; expansion then pulls
        // the linked Quote, NOS and ER even though they don't carry the RFQ id.
        let msgs = synth_chain();
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        f.f_clord = "QR-1".into();
        let r = apply_timeline_filter(&msgs, &idx, &f);
        assert_eq!(r.len(), 4, "exact RFQ seed should expand to the whole chain");
    }

    // ── Other column filters ────────────────────────────────────────────

    #[test]
    fn sender_filter_substring_case_insensitive() {
        let msgs = synth_chain();
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        f.f_sender = "a".into();   // matches sender "A"
        let r = apply_timeline_filter(&msgs, &idx, &f);
        // Two messages have sender=A (RFQ and NOS).
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn target_filter_excludes_non_matches() {
        let msgs = synth_chain();
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        f.f_target = "ZZ".into();
        let r = apply_timeline_filter(&msgs, &idx, &f);
        assert!(r.is_empty());
    }

    #[test]
    fn msg_filter_matches_msg_type_label() {
        let msgs = synth_chain();
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        f.f_msg = "Quote".into();
        let r = apply_timeline_filter(&msgs, &idx, &f);
        // Quote + QuoteRequest both contain "Quote" in their msg_type_label.
        assert!(r.len() >= 1);
    }

    #[test]
    fn detail_filter_matches_symbol_or_text() {
        let msgs = synth_chain();
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        f.f_detail = "AAPL".into();
        let r = apply_timeline_filter(&msgs, &idx, &f);
        assert!(!r.is_empty(), "all msgs carry 55=AAPL");
    }

    #[test]
    fn skip_heartbeats_excludes_session_msgs() {
        let raw = concat!(
            "8=FIX.4.4|9=1|35=A|49=A|56=B|34=1|52=20240101-09:00:00.000|10=000|",
            "8=FIX.4.4|9=1|35=0|49=A|56=B|34=2|52=20240101-09:00:00.001|10=000|",
            "8=FIX.4.4|9=1|35=D|49=A|56=B|34=3|52=20240101-09:00:00.002|11=ORD-X|55=AAPL|10=000|",
        );
        let msgs = parse_all(raw);
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        f.skip_heartbeats = true;
        let r = apply_timeline_filter(&msgs, &idx, &f);
        assert_eq!(r.len(), 1, "only the NOS should remain when heartbeats/logon skipped");
    }

    // ── Time range ──────────────────────────────────────────────────────

    #[test]
    fn time_filter_ge_includes_only_after() {
        let msgs = synth_chain();
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        f.f_time = "2024-01-01 09:00:00.100".into();
        f.f_time_op = ">=".into();
        let r = apply_timeline_filter(&msgs, &idx, &f);
        // Only NOS (.100) and ER (.150) qualify.
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn time_filter_le_includes_only_before() {
        let msgs = synth_chain();
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        f.f_time = "2024-01-01 09:00:00.050".into();
        f.f_time_op = "<=".into();
        let r = apply_timeline_filter(&msgs, &idx, &f);
        // RFQ (.000) + Quote (.050) qualify.
        assert_eq!(r.len(), 2);
    }

    // ── Combined filters ────────────────────────────────────────────────

    #[test]
    fn combined_id_and_msg_filter() {
        let msgs = synth_chain();
        let idx  = build_chain_index(&msgs);
        let mut f = empty_filter();
        f.f_clord = "QR-1".into();        // chain expansion → 4 rows
        // The ER has tag 150=F so its msg_type_label is rewritten to "ER FILL".
        f.f_msg   = "ER FILL".into();
        let r = apply_timeline_filter(&msgs, &idx, &f);
        assert_eq!(r.len(), 1, "chain + msg type intersection should leave just the ER");
    }

    #[test]
    fn results_sorted_newest_first() {
        let msgs = synth_chain();
        let idx  = build_chain_index(&msgs);
        let r = apply_timeline_filter(&msgs, &idx, &empty_filter());
        // Newest first → ER (.150) at top, RFQ (.000) at bottom.
        assert_eq!(msgs[r[0]].msg_type_raw, "8");
        assert_eq!(msgs[*r.last().unwrap()].msg_type_raw, "R");
    }
}
