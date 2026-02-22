use dioxus::prelude::*;

use crate::components::detail::detail_panel;
use crate::components::timeline::timeline_panel;
use crate::model::FixMessage;
use crate::parser::parse_all;
use crate::sample::sample_data;
use crate::style::CSS;

/// Root application component.
pub fn app() -> Element {
    let mut input = use_signal(String::new);
    let mut messages: Signal<Vec<FixMessage>> = use_signal(Vec::new);
    let mut selected_idx: Signal<Option<usize>> = use_signal(|| None);
    let skip_heartbeats = use_signal(|| false);
    let skip_common = use_signal(|| false);

    let mut process = move || {
        let parsed = parse_all(&input.read());
        messages.set(parsed);
        selected_idx.set(None);
    };

    let mut clear = move || {
        input.set(String::new());
        messages.set(Vec::new());
        selected_idx.set(None);
    };

    let mut load_sample = move || {
        let s = sample_data();
        let parsed = parse_all(&s);
        input.set(s);
        messages.set(parsed);
        selected_idx.set(None);
    };

    // Selected message for the detail panel
    let sel = *selected_idx.read();
    let detail_msg: Option<FixMessage> = sel.and_then(|i| messages.read().get(i).cloned());

    rsx! {
        style { {CSS} }
        div { class: "root",
            // ── Toolbar ──
            div { class: "toolbar",
                button { class: "btn btn-process", onclick: move |_| process(), "Process" }
                button { class: "btn btn-clear", onclick: move |_| clear(), "Clear" }
                button { class: "btn btn-sample", onclick: move |_| load_sample(), "Sample data" }
            }

            // ── Textarea ──
            textarea {
                class: "fix-input",
                placeholder: "Paste FIX messages here …",
                value: "{input.read()}",
                oninput: move |evt| input.set(evt.value()),
            }

            // ── Main panels ──
            div { class: "panels",
                timeline_panel {
                    messages: messages,
                    selected_idx: selected_idx,
                    skip_heartbeats: skip_heartbeats,
                }
                detail_panel {
                    detail_msg: detail_msg,
                    skip_common: skip_common,
                }
            }
        }
    }
}
