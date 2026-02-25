use dioxus::prelude::*;

use crate::components::detail::detail_panel;
use crate::components::timeline::timeline_panel;
use crate::model::FixMessage;
use crate::parser::parse_all;
use crate::sample::{sample_data, FIX_SPECS};
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

    let mut load_sample = move |spec: &str| {
        let s = sample_data(spec);
        let parsed = parse_all(&s);
        input.set(s);
        messages.set(parsed);
        selected_idx.set(None);
    };

    let load_file = move || {
        let mut input = input.clone();
        let mut messages = messages.clone();
        let mut selected_idx = selected_idx.clone();
        spawn(async move {
            if let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("FIX log", &["txt", "log", "fix"])
                .add_filter("All files", &["*"])
                .pick_file()
                .await
            {
                let bytes = file.read().await;
                let content = String::from_utf8_lossy(&bytes).to_string();
                let parsed = parse_all(&content);
                input.set(content);
                messages.set(parsed);
                selected_idx.set(None);
            }
        });
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
                button { class: "btn btn-load", onclick: move |_| load_file(), "Load file" }
                span { class: "sample-label", "Sample: " }
                {FIX_SPECS.iter().enumerate().map(|(i, spec)| {
                    let sep = if i > 0 { rsx! { span { class: "sample-sep", " | " } } } else { rsx! { } };
                    rsx! {
                        {sep}
                        button {
                            class: "btn btn-sample-inline",
                            onclick: move |_| load_sample(spec),
                            "{spec}"
                        }
                    }
                })}
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
