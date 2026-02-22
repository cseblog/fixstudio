use dioxus::prelude::*;

use crate::dictionary::badge_class;
use crate::model::FixMessage;

/// Build a one-line detail summary (side / qty / symbol / text).
fn build_detail_text(m: &FixMessage) -> String {
    let mut parts = Vec::new();
    if !m.side.is_empty() {
        parts.push(m.side.clone());
    }
    if !m.order_qty.is_empty() {
        parts.push(m.order_qty.clone());
    }
    if !m.symbol.is_empty() {
        parts.push(m.symbol.clone());
    }
    if !m.text.is_empty() {
        parts.push(m.text.clone());
    }
    parts.join("  ")
}

/// Renders the Timeline panel (left side).
#[component]
pub fn timeline_panel(
    messages: Signal<Vec<FixMessage>>,
    selected_idx: Signal<Option<usize>>,
    skip_heartbeats: Signal<bool>,
) -> Element {
    let skip_hb = *skip_heartbeats.read();
    let msgs = messages.read();
    let sel = *selected_idx.read();

    let timeline_indices: Vec<usize> = msgs
        .iter()
        .enumerate()
        .filter(|(_, m)| !(skip_hb && (m.msg_type_raw == "0" || m.msg_type_raw == "A")))
        .map(|(i, _)| i)
        .collect();

    rsx! {
        div { class: "panel-timeline",
            div { class: "panel-header",
                h2 { "Timeline" }
                label { class: "check-label",
                    input {
                        r#type: "checkbox",
                        checked: *skip_heartbeats.read(),
                        onchange: move |evt: Event<FormData>| skip_heartbeats.set(evt.checked()),
                    }
                    " Skip heartbeats"
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
                div { class: "tbl-body",
                    for idx in timeline_indices.iter() {
                        {
                            let i = *idx;
                            let m = &msgs[i];
                            let is_sel = sel == Some(i);
                            let badge_cls = badge_class(&m.msg_type_raw);
                            let detail_text = build_detail_text(m);
                            rsx! {
                                div {
                                    class: if is_sel { "tbl-row tbl-timeline-row row-selected" } else { "tbl-row tbl-timeline-row" },
                                    onclick: move |_| selected_idx.set(Some(i)),
                                    span { class: "cell-time", "{m.time}" }
                                    span { "{m.sender}" }
                                    span { "{m.target}" }
                                    span { span { class: "badge {badge_cls}", "{m.msg_type_label}" } }
                                    span { "{m.cl_ord_id}" }
                                    span { class: "cell-detail", "{detail_text}" }
                                }
                            }
                        }
                    }
                    if timeline_indices.is_empty() {
                        div { class: "empty-state", "No messages parsed yet." }
                    }
                }
            }
        }
    }
}
