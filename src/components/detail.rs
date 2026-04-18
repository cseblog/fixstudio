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
        let td = tag_description(field.tag);
        let name = if td != "Unknown" {
            format!("{}({})", td, field.tag)
        } else {
            field.tag.to_string()
        };
        let val = field.value_in(&msg.arena);
        let val_desc = value_description(field.tag, val);
        if val_desc.is_empty() {
            lines.push(format!("  {}: {}", name, val));
        } else {
            lines.push(format!("  {}: {} [{}]", name, val, val_desc));
        }
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
        let val = f.value_in(&msg.arena);
        let mut e = format!(
            "  {{\"tag\": \"{}\", \"name\": \"{}\", \"value\": \"{}\"",
            f.tag,
            json_escape(tag_description(f.tag)),
            json_escape(val),
        );
        let val_desc = value_description(f.tag, val);
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

#[component]
pub fn detail_panel(
    detail_msg: Option<FixMessage>,
    skip_common: Signal<bool>,
    selected_idx: Signal<Option<usize>>,
) -> Element {
    let mut view         = use_signal(|| VIEW_TABLE);
    let mut copied       = use_signal(|| false);
    let mut table_filter = use_signal(String::new);

    // Reset "Copied!" state whenever the selected message changes.
    use_effect(move || {
        let _ = selected_idx.read();
        copied.set(false);
    });

    // Only build text for the active tab — no point computing both up front.
    let view_now  = *view.read();
    let raw_text  = if view_now == VIEW_RAW  { detail_msg.as_ref().map(|m| build_raw_text(m))  } else { None };
    let json_text = if view_now == VIEW_JSON { detail_msg.as_ref().map(|m| build_json_text(m)) } else { None };

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
                    div { class: "detail-filter-row",
                        input {
                            class: "detail-filter-input",
                            r#type: "text",
                            placeholder: "Filter by tag, name, value…",
                            value: "{table_filter}",
                            oninput: move |e| table_filter.set(e.value()),
                        }
                    }
                    div { class: "tbl-header tbl-detail-row",
                        span { "Tag" }
                        span { "Tag Description" }
                        span { "Value" }
                        span { "Value Description" }
                    }
                    div { class: "tbl-body",
                        if let Some(ref msg) = detail_msg {
                            {
                                let skip   = *skip_common.read();
                                let filter = table_filter.read().to_lowercase();
                                rsx! {
                                    for field in msg.fields.iter().filter(|f| {
                                        if skip && is_common_tag(f.tag) { return false; }
                                        if filter.is_empty() { return true; }
                                        let val      = f.value_in(&msg.arena);
                                        let val_desc = value_description(f.tag, val);
                                        f.tag.to_string().contains(filter.as_str())
                                        || tag_description(f.tag).to_lowercase().contains(filter.as_str())
                                        || val.to_lowercase().contains(filter.as_str())
                                        || val_desc.to_lowercase().contains(filter.as_str())
                                    }) {
                                        div { class: "tbl-row tbl-detail-row",
                                            span { class: "tag-num", "{field.tag}" }
                                            span { span { class: "badge {tag_badge_class(field.tag)}", "{tag_description(field.tag)}" } }
                                            span { "{field.value_in(&msg.arena)}" }
                                            span { "{value_description(field.tag, field.value_in(&msg.arena))}" }
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
