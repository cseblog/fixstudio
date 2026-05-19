use dioxus::prelude::*;
use dioxus::document::eval;

use crate::dictionary::{is_common_tag, tag_badge_class, tag_description, value_description};
use crate::model::FixMessage;

// ── View mode constants ───────────────────────────────────────────────────────
const VIEW_TABLE: u8 = 0;
const VIEW_RAW:   u8 = 1;

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
    // Per-tab Detail UI state — preserved across tab switches.
    mut view_kind:    Signal<u8>,
    mut table_filter: Signal<String>,
    mut filter_open:  Signal<bool>,
) -> Element {
    let mut copied = use_signal(|| false);

    // Reset "Copied!" state whenever the selected message changes.
    use_effect(move || {
        let _ = selected_idx.read();
        copied.set(false);
    });

    // Only build text for the active view — no point computing if hidden.
    let view_now  = *view_kind.read();
    let raw_text  = if view_now == VIEW_RAW { detail_msg.as_ref().map(|m| build_raw_text(m)) } else { None };

    rsx! {
        div { class: "panel-detail",
            // ── Single inline strip: label · view tabs · filter · common ──
            //   Mirrors the timeline's stats-strip so both panels share visual rhythm.
            div { class: "stats-strip detail-strip",
                span { class: "panel-tag", "Detail" }
                if detail_msg.is_some() {
                    div { class: "seg-tabs",
                        button {
                            class: if *view_kind.read() == VIEW_TABLE { "seg-tab seg-tab-active" } else { "seg-tab" },
                            onclick: move |_| view_kind.set(VIEW_TABLE),
                            "Table"
                        }
                        button {
                            class: if *view_kind.read() == VIEW_RAW { "seg-tab seg-tab-active" } else { "seg-tab" },
                            onclick: move |_| { view_kind.set(VIEW_RAW); copied.set(false); },
                            "Raw"
                        }
                    }
                }
                div { class: "stats-spacer" }
                if detail_msg.is_some() {
                    button {
                        class: if *filter_open.read() || !table_filter.read().is_empty() { "btn-icon btn-icon-on" } else { "btn-icon" },
                        title: "Filter fields",
                        onclick: move |_| { let v = !*filter_open.read(); filter_open.set(v); },
                        "⏷ Filter"
                    }
                }
                button {
                    class: if *skip_common.read() { "btn-icon btn-icon-on" } else { "btn-icon" },
                    title: "Hide common fields (BeginString, BodyLength, Sender/Target, Time, Checksum…)",
                    onclick: move |_| { let v = !*skip_common.read(); skip_common.set(v); },
                    "⊘ Common"
                }
            }

            // ── Table view ──
            if *view_kind.read() == VIEW_TABLE {
                div { class: "table-wrap",
                    if *filter_open.read() || !table_filter.read().is_empty() {
                        div { class: "detail-filter-row",
                            input {
                                class: "detail-filter-input",
                                r#type: "text",
                                placeholder: "Filter by tag, name, value…",
                                value: "{table_filter}",
                                oninput: move |e| table_filter.set(e.value()),
                            }
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
                            div { class: "empty-state",
                                span { class: "empty-state-icon",  "👈" }
                                span { class: "empty-state-title", "Pick a message" }
                                span { class: "empty-state-hint",
                                    "Click any row in the timeline to inspect its FIX tags here."
                                }
                            }
                        }
                    }
                }
            }

            // ── Raw Text view ──
            if *view_kind.read() == VIEW_RAW {
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

        }
    }
}
