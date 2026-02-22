use dioxus::prelude::*;
use dioxus::document::eval;

use crate::dictionary::{is_common_tag, tag_badge_class};
use crate::model::{FixField, FixMessage};

/// Build a copyable raw-text representation of the message.
fn build_raw_text(msg: &FixMessage) -> String {
    let mut lines = Vec::new();

    // Header: message type label
    lines.push(format!("{}>>", msg.msg_type_label));

    for field in &msg.fields {
        let name = if field.tag_description != "Unknown" {
            format!("{}({})", field.tag_description, field.tag)
        } else {
            field.tag.clone()
        };

        let val_part = if field.value_description.is_empty() {
            field.value.clone()
        } else {
            format!("{} [{}]", field.value, field.value_description)
        };

        lines.push(format!("  {}: {}", name, val_part));
    }

    lines.join("\n")
}

/// Renders the Detail panel (right side).
#[component]
pub fn detail_panel(
    detail_msg: Option<FixMessage>,
    skip_common: Signal<bool>,
) -> Element {
    let mut show_raw = use_signal(|| false);
    let mut copied = use_signal(|| false);

    // Build raw text for the current message (if any)
    let raw_text = detail_msg.as_ref().map(|m| build_raw_text(m));

    rsx! {
        div { class: "panel-detail",
            // ── Header row ──
            div { class: "panel-header",
                h2 { "Detail" }
                div { class: "header-actions",
                    label { class: "check-label",
                        input {
                            r#type: "checkbox",
                            checked: *skip_common.read(),
                            onchange: move |evt: Event<FormData>| skip_common.set(evt.checked()),
                        }
                        " Skip common fields"
                    }
                }
            }

            // ── View toggle tabs ──
            if detail_msg.is_some() {
                div { class: "view-tabs",
                    button {
                        class: if !*show_raw.read() { "tab-btn tab-active" } else { "tab-btn" },
                        onclick: move |_| show_raw.set(false),
                        "Table"
                    }
                    button {
                        class: if *show_raw.read() { "tab-btn tab-active" } else { "tab-btn" },
                        onclick: move |_| { show_raw.set(true); copied.set(false); },
                        "Raw Text"
                    }
                }
            }

            // ── Table view ──
            if !*show_raw.read() {
                div { class: "table-wrap",
                    div { class: "tbl-header tbl-detail-row",
                        span { "Tag" }
                        span { "Tag Description" }
                        span { "Value" }
                        span { "Value Description" }
                    }
                    div { class: "tbl-body",
                        if let Some(ref msg) = detail_msg {
                            {
                                let skip = *skip_common.read();
                                let fields: Vec<&FixField> = msg.fields.iter()
                                    .filter(|f| !(skip && is_common_tag(&f.tag)))
                                    .collect();
                                rsx! {
                                    for field in fields.iter() {
                                        {
                                            let desc_cls = tag_badge_class(&field.tag);
                                            rsx! {
                                                div { class: "tbl-row tbl-detail-row",
                                                    span { class: "tag-num", "{field.tag}" }
                                                    span { span { class: "badge {desc_cls}", "{field.tag_description}" } }
                                                    span { "{field.value}" }
                                                    span { "{field.value_description}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            div { class: "empty-state", "Click a message in the timeline to view its fields." }
                        }
                    }
                }
            }

            // ── Raw Text view ──
            if *show_raw.read() {
                if let Some(ref text) = raw_text {
                    div { class: "raw-text-wrap",
                        div { class: "raw-text-toolbar",
                            button {
                                class: if *copied.read() { "btn btn-copied" } else { "btn btn-copy" },
                                onclick: {
                                    let t = text.clone();
                                    move |_| {
                                        // Use the clipboard API via eval
                                        let js = format!(
                                            "navigator.clipboard.writeText(`{}`)",
                                            t.replace('\\', "\\\\").replace('`', "\\`")
                                        );
                                        eval(&js);
                                        copied.set(true);
                                    }
                                },
                                if *copied.read() { "Copied!" } else { "Copy" }
                            }
                        }
                        pre { class: "raw-text", "{text}" }
                    }
                }
            }
        }
    }
}
