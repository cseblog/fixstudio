use dioxus::prelude::*;
use dioxus::document::eval;

use crate::dictionary::{is_common_tag, tag_badge_class, tag_description, value_description};
use crate::model::FixMessage;

// ── View mode constants ───────────────────────────────────────────────────────
const VIEW_TABLE: u8 = 0;
const VIEW_RAW:   u8 = 1;
const VIEW_JSON:  u8 = 2;

// ── Text builders ─────────────────────────────────────────────────────────────

fn build_raw_text(msg: &FixMessage) -> String {
    let mut lines = Vec::new();
    lines.push(format!("{}>>", msg.msg_type_label));
    for field in &msg.fields {
        let td = tag_description(&field.tag);
        let name = if td != "Unknown" {
            format!("{}({})", td, field.tag)
        } else {
            field.tag.to_string()
        };
        let val_desc = value_description(&field.tag, &field.value);
        let val_part = if val_desc.is_empty() {
            field.value.to_string()
        } else {
            format!("{} [{}]", field.value, val_desc)
        };
        lines.push(format!("  {}: {}", name, val_part));
    }
    lines.join("\n")
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"',  "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
     .replace('\t', "\\t")
}

/// Serialise all fields (skip_common not applied — JSON is a full export).
fn build_json_text(msg: &FixMessage) -> String {
    let entries: Vec<String> = msg.fields.iter().map(|f| {
        let mut e = format!(
            "  {{\"tag\": \"{}\", \"name\": \"{}\", \"value\": \"{}\"",
            json_escape(&f.tag),
            json_escape(tag_description(&f.tag)),
            json_escape(&f.value),
        );
        let val_desc = value_description(&f.tag, &f.value);
        if !val_desc.is_empty() {
            e.push_str(&format!(", \"decoded\": \"{}\"", json_escape(&val_desc)));
        }
        e.push('}');
        e
    }).collect();

    format!("[\n{}\n]", entries.join(",\n"))
}

// ── Copy helper ───────────────────────────────────────────────────────────────

fn copy_js(text: &str) -> String {
    format!(
        "navigator.clipboard.writeText(`{}`)",
        text.replace('\\', "\\\\").replace('`', "\\`")
    )
}

// ── Component ─────────────────────────────────────────────────────────────────

/// Renders the Detail panel (right side).
#[component]
pub fn detail_panel(
    detail_msg: Option<FixMessage>,
    skip_common: Signal<bool>,
) -> Element {
    let mut view   = use_signal(|| VIEW_TABLE);
    let mut copied = use_signal(|| false);

    let raw_text  = detail_msg.as_ref().map(|m| build_raw_text(m));
    let json_text = detail_msg.as_ref().map(|m| build_json_text(m));

    rsx! {
        div { class: "panel-detail",
            // ── Header ──
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

            // ── Tabs (only when a message is selected) ──
            if detail_msg.is_some() {
                div { class: "view-tabs",
                    button {
                        class: if *view.read() == VIEW_TABLE { "tab-btn tab-active" } else { "tab-btn" },
                        onclick: move |_| view.set(VIEW_TABLE),
                        "Table"
                    }
                    button {
                        class: if *view.read() == VIEW_RAW { "tab-btn tab-active" } else { "tab-btn" },
                        onclick: move |_| { view.set(VIEW_RAW); copied.set(false); },
                        "Raw Text"
                    }
                    button {
                        class: if *view.read() == VIEW_JSON { "tab-btn tab-active" } else { "tab-btn" },
                        onclick: move |_| { view.set(VIEW_JSON); copied.set(false); },
                        "JSON"
                    }
                }
            }

            // ── Table view ──
            if *view.read() == VIEW_TABLE {
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
                                rsx! {
                                    for field in msg.fields.iter().filter(|f| !(skip && is_common_tag(&f.tag))) {
                                        div { class: "tbl-row tbl-detail-row",
                                            span { class: "tag-num", "{field.tag}" }
                                            span { span { class: "badge {tag_badge_class(&field.tag)}", "{tag_description(&field.tag)}" } }
                                            span { "{field.value}" }
                                            span { "{value_description(&field.tag, &field.value)}" }
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
            if *view.read() == VIEW_RAW {
                if let Some(ref text) = raw_text {
                    div { class: "raw-text-wrap",
                        div { class: "raw-text-toolbar",
                            button {
                                class: if *copied.read() { "btn btn-copied" } else { "btn btn-copy" },
                                onclick: {
                                    let js = copy_js(text);
                                    move |_| { eval(&js); copied.set(true); }
                                },
                                if *copied.read() { "Copied!" } else { "Copy" }
                            }
                        }
                        pre { class: "raw-text", "{text}" }
                    }
                }
            }

            // ── JSON view ──
            if *view.read() == VIEW_JSON {
                if let Some(ref text) = json_text {
                    div { class: "raw-text-wrap",
                        div { class: "raw-text-toolbar",
                            button {
                                class: if *copied.read() { "btn btn-copied" } else { "btn btn-copy" },
                                onclick: {
                                    let js = copy_js(text);
                                    move |_| { eval(&js); copied.set(true); }
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
