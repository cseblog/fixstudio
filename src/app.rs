use std::time::Instant;
use std::mem;

use dioxus::prelude::*;

use crate::components::detail::detail_panel;
use crate::components::timeline::timeline_panel;
use crate::model::FixMessage;
use crate::parser::parse_all;
use crate::sample::{sample_data, FIX_SPECS};
use crate::style::CSS;

/// Replace `signal`'s value with `new_data`, then drop the old `Vec<FixMessage>`
/// on a background thread so the main UI thread isn't blocked by deallocation
/// of potentially millions of structs.
fn offload_replace(signal: &mut Signal<Vec<FixMessage>>, new_data: Vec<FixMessage>) {
    let old = mem::replace(&mut *signal.write(), new_data);
    if !old.is_empty() {
        std::thread::spawn(move || drop(old));
    }
}

/// Root application component.
pub fn app() -> Element {
    let mut input = use_signal(String::new);
    let mut messages: Signal<Vec<FixMessage>> = use_signal(Vec::new);
    let mut selected_idx: Signal<Option<usize>> = use_signal(|| None);
    let skip_heartbeats = use_signal(|| true);
    let skip_common = use_signal(|| false);
    let mut parse_stats: Signal<Option<(usize, u64)>> = use_signal(|| None);
    let loading = use_signal(|| false);
    // Tracks the last file loaded via the file dialog (name only — not the content).
    let mut file_name: Signal<Option<String>> = use_signal(|| None);

    let mut process = move || {
        let t = Instant::now();
        let parsed = parse_all(&input.read());
        let ms = t.elapsed().as_micros() as u64;
        parse_stats.set(Some((parsed.len(), ms)));
        offload_replace(&mut messages, parsed);
        selected_idx.set(None);
        file_name.set(None);
    };

    let mut clear = move || {
        input.set(String::new());
        offload_replace(&mut messages, Vec::new());
        selected_idx.set(None);
        parse_stats.set(None);
        file_name.set(None);
    };

    let mut load_sample = move |spec: &str| {
        let s = sample_data(spec);
        let t = Instant::now();
        let parsed = parse_all(&s);
        let ms = t.elapsed().as_micros() as u64;
        parse_stats.set(Some((parsed.len(), ms)));
        input.set(s);
        offload_replace(&mut messages, parsed);
        selected_idx.set(None);
        file_name.set(None);
    };

    let load_file = move || {
        let mut messages    = messages.clone();
        let mut selected_idx = selected_idx.clone();
        let mut parse_stats = parse_stats.clone();
        let mut loading     = loading.clone();
        let mut file_name   = file_name.clone();
        spawn(async move {
            loading.set(true);
            if let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("FIX log", &["txt", "log", "fix"])
                .add_filter("All files", &["*"])
                .pick_file()
                .await
            {
                let name  = file.file_name();
                let bytes = file.read().await;
                // Use from_utf8_lossy but do NOT store the content in a Signal —
                // 135 MB in a reactive signal serialises to the WebView on every render.
                let content = String::from_utf8_lossy(&bytes);
                let t = Instant::now();
                let parsed = parse_all(&content);
                let ms = t.elapsed().as_micros() as u64;
                // `content` (and `bytes`) are dropped here — not kept in any signal.
                parse_stats.set(Some((parsed.len(), ms)));
                offload_replace(&mut messages, parsed);
                selected_idx.set(None);
                file_name.set(Some(name));
            }
            loading.set(false);
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
                // "Process" only makes sense when the user has pasted text manually.
                if file_name.read().is_none() {
                    button { class: "btn btn-process", onclick: move |_| process(), "Process" }
                }
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

            // ── Input area ──
            if *loading.read() {
                div { class: "fix-loading", "Loading file and parsing messages…" }
            } else if let Some(ref name) = *file_name.read() {
                // File was loaded via dialog — don't put 135 MB in the textarea.
                div { class: "fix-file-banner",
                    span { class: "fix-file-icon", "📂" }
                    span { class: "fix-file-name", "{name}" }
                    span { class: "fix-file-hint", "— click Clear to reset" }
                }
            } else {
                textarea {
                    class: "fix-input",
                    placeholder: "Paste FIX messages here …",
                    value: "{input.read()}",
                    oninput: move |evt| input.set(evt.value()),
                }
            }

            // ── Main panels ──
            div { class: "panels",
                timeline_panel {
                    messages: messages,
                    selected_idx: selected_idx,
                    skip_heartbeats: skip_heartbeats,
                    parse_stats: parse_stats,
                }
                detail_panel {
                    detail_msg: detail_msg,
                    skip_common: skip_common,
                }
            }
        }
    }
}
