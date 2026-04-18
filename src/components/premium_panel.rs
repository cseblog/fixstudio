use dioxus::prelude::*;
use dioxus::document::eval;

use crate::types::ViewMode;

/// Right-side features panel.
#[component]
pub fn premium_panel(
    has_messages: bool,
    view_mode: Signal<ViewMode>,
    right_panel_collapsed: Signal<bool>,
    right_panel_width: Signal<f64>,
    left_collapsed: bool,
) -> Element {
    let in_lifecycle    = *view_mode.read() == ViewMode::Lifecycle;
    let in_overview     = *view_mode.read() == ViewMode::Overview;
    let in_validator    = *view_mode.read() == ViewMode::Validator;
    let right_collapsed = *right_panel_collapsed.read();
    let right_w         = *right_panel_width.read() as u32;

    let outer_style = if right_collapsed {
        "width: 0; min-width: 0; overflow: hidden; border: none;".to_string()
    } else if left_collapsed {
        "flex: 1; min-width: 0;".to_string()
    } else {
        format!("flex-shrink: 0; width: {right_w}px; min-width: 0;")
    };

    rsx! {
        div {
            id: "premium-panel-main",
            class: "premium-panel",
            style: "{outer_style}",

            div { class: "premium-panel-header",
                span { class: "premium-panel-title", "Features" }
                button {
                    class: "panel-collapse-btn",
                    title: "Collapse panel",
                    onclick: move |_| {
                        let mut rpw = right_panel_width;
                        let mut rpc = right_panel_collapsed;
                        spawn(async move {
                            if let Ok(v) = eval(r#"(function(){
                                var p = document.getElementById('premium-panel-main');
                                var w = parseFloat(p.style.width);
                                return (w > 50) ? w : p.getBoundingClientRect().width;
                            })()"#).await {
                                if let Some(w) = v.as_f64() {
                                    if w > 50.0 { rpw.set(w); }
                                }
                            }
                            rpc.set(true);
                        });
                    },
                    "›"
                }
            }

            div { class: "premium-panel-scroll",

                div { class: "feature-card",
                    div { class: "feature-card-top",
                        span { class: "feature-card-name", "Timeline" }
                        span { class: "badge badge-gray feature-badge", "Active" }
                    }
                    p { class: "feature-card-desc", "Browse and inspect every FIX message." }
                }

                div { class: "feature-card",
                    div { class: "feature-card-top",
                        span { class: "feature-card-name", "Latency Analysis" }
                        span { class: "badge badge-gray feature-badge", "Active" }
                    }
                    p { class: "feature-card-desc",
                        "Reconstruct full order chains from RFQ to fill, with latency at each hop."
                    }
                    if has_messages {
                        button {
                            class: "btn-feature",
                            onclick: move |_| {
                                view_mode.set(if in_lifecycle {
                                    ViewMode::Timeline
                                } else {
                                    ViewMode::Lifecycle
                                });
                            },
                            if in_lifecycle { "← Back to Timeline" } else { "View Lifecycle →" }
                        }
                    } else {
                        span { class: "feature-card-hint", "Load data to use" }
                    }
                }

                div { class: "feature-card",
                    div { class: "feature-card-top",
                        span { class: "feature-card-name", "Session Analysis" }
                        span { class: "badge badge-gray feature-badge", "Active" }
                    }
                    p { class: "feature-card-desc",
                        "Fill quality scorecard, session health diagnostics, \
                        and an executive session summary."
                    }
                    if has_messages {
                        button {
                            class: "btn-feature",
                            onclick: move |_| {
                                view_mode.set(if in_overview {
                                    ViewMode::Timeline
                                } else {
                                    ViewMode::Overview
                                });
                            },
                            if in_overview { "← Back to Timeline" } else { "View Report →" }
                        }
                    } else {
                        span { class: "feature-card-hint", "Load data to use" }
                    }
                }

                div { class: "feature-card",
                    div { class: "feature-card-top",
                        span { class: "feature-card-name", "FIX Validator" }
                        span { class: "badge badge-gray feature-badge", "Active" }
                    }
                    p { class: "feature-card-desc",
                        "Validate messages against FIX spec, check required tags, \
                        enums, checksums & consistency rules."
                    }
                    if has_messages {
                        button {
                            class: "btn-feature",
                            onclick: move |_| {
                                view_mode.set(if in_validator {
                                    ViewMode::Timeline
                                } else {
                                    ViewMode::Validator
                                });
                            },
                            if in_validator { "← Back to Timeline" } else { "Open Validator →" }
                        }
                    } else {
                        span { class: "feature-card-hint", "Load data to use" }
                    }
                }

                div { class: "feature-card feature-card-soon",
                    div { class: "feature-card-top",
                        span { class: "feature-card-name", "Order Flow & AI Diagnostics" }
                        span { class: "badge badge-orange feature-badge", "Soon" }
                    }
                    p { class: "feature-card-desc",
                        "Detect TWAP, VWAP, iceberg & spoofing patterns. \
                        AI-powered reject root-cause analysis with suggested fixes."
                    }
                }

                div { class: "feature-card feature-card-soon",
                    div { class: "feature-card-top",
                        span { class: "feature-card-name", "AI FIX Builder" }
                        span { class: "badge badge-orange feature-badge", "Soon" }
                    }
                    p { class: "feature-card-desc",
                        "Generate FIX engine client or server code tailored to your spec."
                    }
                }
            }
        }
    }
}
