use dioxus::prelude::*;

use crate::sample::FIX_SPECS;

/// Hero landing screen shown before any data is loaded. Acts as the welcome
/// surface — branded perf stats up top, then direct CTAs (Load / Sample),
/// with the textarea tucked underneath as the "paste" path.
#[component]
pub fn Hero(
    on_load_file:   EventHandler<()>,
    on_load_folder: EventHandler<()>,
    on_load_sample: EventHandler<String>,
    recent_files:   Vec<(String, String)>,
    on_open_recent: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "hero",
            div { class: "hero-headline",
                h1 { class: "hero-title-text", "Drop a FIX log to begin" }
                p { class: "hero-sub", "Browse, inspect, validate and compare at native speed." }
            }

            div { class: "hero-cta-grid",
                button {
                    class: "hero-cta hero-cta-primary",
                    onclick: move |_| on_load_file.call(()),
                    span { class: "hero-cta-icon", "📄" }
                    span { class: "hero-cta-label", "Load file" }
                    span { class: "hero-cta-hint", "⌘O" }
                }
                button {
                    class: "hero-cta",
                    onclick: move |_| on_load_folder.call(()),
                    span { class: "hero-cta-icon", "📁" }
                    span { class: "hero-cta-label", "Load folder" }
                    span { class: "hero-cta-hint", "⌘⇧O" }
                }
            }

            if !recent_files.is_empty() {
                div { class: "hero-section",
                    div { class: "hero-section-label", "Recent" }
                    div { class: "hero-recent-list",
                        for (path, name) in recent_files.iter() {
                            {
                                let p = path.clone();
                                rsx! {
                                    button {
                                        class: "hero-recent-chip",
                                        title: "{p}",
                                        onclick: move |_| on_open_recent.call(p.clone()),
                                        "{name}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "hero-section",
                div { class: "hero-section-label", "Samples" }
                div { class: "hero-sample-list",
                    for spec in FIX_SPECS.iter() {
                        {
                            let s = spec.to_string();
                            rsx! {
                                button {
                                    class: "hero-sample-chip",
                                    onclick: move |_| on_load_sample.call(s.clone()),
                                    "{spec}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
