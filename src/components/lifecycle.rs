use dioxus::prelude::*;
use std::collections::HashMap;

use crate::dictionary::badge_class;
use crate::model::FixMessage;

#[derive(Clone, PartialEq)]
struct OrderGroup {
    cl_ord_id: String,
    symbol: String,
    side: String,
    order_qty: String,
    first_time: String,
    last_type: &'static str,
    msg_indices: Vec<usize>,
}

fn build_groups(messages: &[FixMessage]) -> Vec<OrderGroup> {
    let mut map: HashMap<String, OrderGroup> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    for (i, msg) in messages.iter().enumerate() {
        if msg.cl_ord_id.is_empty() {
            continue;
        }
        let key = msg.cl_ord_id.to_string();
        let entry = map.entry(key.clone()).or_insert_with(|| {
            order.push(key.clone());
            OrderGroup {
                cl_ord_id: msg.cl_ord_id.to_string(),
                symbol: msg.symbol.to_string(),
                side: msg.side.to_string(),
                order_qty: msg.order_qty.to_string(),
                first_time: msg.time.to_string(),
                last_type: msg.msg_type_label,
                msg_indices: Vec::new(),
            }
        });
        entry.msg_indices.push(i);
        entry.last_type = msg.msg_type_label;
        if entry.symbol.is_empty() && !msg.symbol.is_empty() {
            entry.symbol = msg.symbol.to_string();
        }
        if entry.side.is_empty() && !msg.side.is_empty() {
            entry.side = msg.side.to_string();
        }
    }

    order.into_iter().filter_map(|k| map.remove(&k)).collect()
}

#[component]
pub fn lifecycle_panel(
    messages: Signal<Vec<FixMessage>>,
    selected_idx: Signal<Option<usize>>,
) -> Element {
    let groups = use_memo(move || build_groups(&messages.read()));
    let mut selected_group: Signal<Option<String>> = use_signal(|| None);

    let group_list = groups.read();
    let selected_id = selected_group.read().clone();

    // Resolve detail messages for the selected group
    let detail_msgs: Vec<(usize, FixMessage)> = {
        let msgs = messages.read();
        if let Some(ref id) = selected_id {
            if let Some(g) = group_list.iter().find(|g| &g.cl_ord_id == id) {
                g.msg_indices
                    .iter()
                    .filter_map(|&i| msgs.get(i).map(|m| (i, m.clone())))
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    };

    rsx! {
        div { class: "panels",
            // ── Left: order groups ──
            div { class: "panel-timeline",
                div { class: "panel-header",
                    div { class: "panel-title",
                        h2 { "Trade Lifecycle" }
                        span { class: "parse-stats", "{group_list.len()} orders" }
                    }
                }
                div { class: "table-wrap",
                    div { class: "tbl-header",
                        div { class: "tbl-lc-row",
                            span { "ClOrdID" }
                            span { "Symbol" }
                            span { "Side" }
                            span { "Qty" }
                            span { "Last Type" }
                            span { "#" }
                            span { "First Time" }
                        }
                    }
                    div { class: "tbl-body",
                        {group_list.iter().map(|g| {
                            let id = g.cl_ord_id.clone();
                            let is_sel = selected_id.as_deref() == Some(&id);
                            let cl_ord_id = g.cl_ord_id.clone();
                            let symbol = g.symbol.clone();
                            let side = g.side.clone();
                            let qty = g.order_qty.clone();
                            let last_type = g.last_type;
                            let count = g.msg_indices.len();
                            let first_time = g.first_time.clone();
                            let badge = badge_class(last_type);
                            rsx! {
                                div {
                                    class: if is_sel { "tbl-row tbl-lc-row row-selected" } else { "tbl-row tbl-lc-row" },
                                    onclick: move |_| {
                                        if is_sel {
                                            selected_group.set(None);
                                        } else {
                                            selected_group.set(Some(id.clone()));
                                        }
                                    },
                                    span { class: "lc-clordid", "{cl_ord_id}" }
                                    span { class: "lc-symbol", "{symbol}" }
                                    span { class: "lc-side", "{side}" }
                                    span { class: "lc-qty", "{qty}" }
                                    span { class: "badge {badge}", "{last_type}" }
                                    span { class: "lc-count", "{count}" }
                                    span { class: "lc-time", "{first_time}" }
                                }
                            }
                        })}
                    }
                }
            }

            // ── Right: messages in selected order ──
            div { class: "panel-detail",
                div { class: "panel-header",
                    div { class: "panel-title",
                        h2 { "Order Chain" }
                        if let Some(ref id) = selected_id {
                            span { class: "lc-selected-id", "{id}" }
                        }
                    }
                }
                div { class: "table-wrap",
                    div { class: "tbl-header",
                        div { class: "tbl-lc-chain-row",
                            span { "#" }
                            span { "Time" }
                            span { "Type" }
                            span { "Text / Info" }
                        }
                    }
                    div { class: "tbl-body",
                        if detail_msgs.is_empty() {
                            div { class: "lc-empty",
                                if selected_id.is_none() {
                                    "← Select an order to see its message chain"
                                } else {
                                    "No messages"
                                }
                            }
                        } else {
                            {detail_msgs.iter().enumerate().map(|(seq, (orig_idx, msg))| {
                                let orig = *orig_idx;
                                let time = msg.time.to_string();
                                let label = msg.msg_type_label;
                                let badge = badge_class(label);
                                let info = {
                                    let mut parts = Vec::new();
                                    if !msg.side.is_empty()      { parts.push(msg.side.to_string()); }
                                    if !msg.order_qty.is_empty() { parts.push(msg.order_qty.to_string()); }
                                    if !msg.symbol.is_empty()    { parts.push(msg.symbol.to_string()); }
                                    if !msg.text.is_empty()      { parts.push(msg.text.to_string()); }
                                    parts.join("  ")
                                };
                                let is_sel = *selected_idx.read() == Some(orig);
                                rsx! {
                                    div {
                                        class: if is_sel { "tbl-row tbl-lc-chain-row row-selected" } else { "tbl-row tbl-lc-chain-row" },
                                        onclick: move |_| selected_idx.set(Some(orig)),
                                        span { class: "lc-seq", "{seq + 1}" }
                                        span { class: "lc-time", "{time}" }
                                        span { class: "badge {badge}", "{label}" }
                                        span { class: "lc-info", "{info}" }
                                    }
                                }
                            })}
                        }
                    }
                }
            }
        }
    }
}
