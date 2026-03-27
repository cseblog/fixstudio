use dioxus::prelude::*;
use dioxus::document::eval;

use crate::license::{clear_license, instance_name, save_license, validate_license, StoredLicense};
use crate::types::ViewMode;

const CHECKOUT_URL: &str = "https://whop.com/checkout/plan_HggsqwuvlHbIu";

/// Right-side Pro features panel with license activation.
#[component]
pub fn premium_panel(
    is_pro: Signal<bool>,
    has_messages: bool,
    view_mode: Signal<ViewMode>,
    right_panel_collapsed: Signal<bool>,
    right_panel_width: Signal<f64>,
    left_collapsed: bool,
) -> Element {
    let mut license_input                       = use_signal(String::new);
    let mut activate_error: Signal<Option<String>> = use_signal(|| None);
    let mut activate_loading                    = use_signal(|| false);
    let mut show_activate                       = use_signal(|| false);

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
                span { class: "premium-panel-title premium-panel-title-pro", "Pro Features" }
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

                div { class: "feature-tier-label", "Free" }
                div { class: "feature-card",
                    div { class: "feature-card-top",
                        span { class: "feature-card-name", "Timeline" }
                        span { class: "badge badge-gray feature-badge", "Active" }
                    }
                    p { class: "feature-card-desc", "Browse and inspect every FIX message." }
                }

                div { class: "feature-tier-label feature-tier-label-premium", "Pro" }

                if *is_pro.read() {
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
                    div { class: "license-deactivate-wrap",
                        button {
                            class: "btn-deactivate",
                            onclick: move |_| {
                                clear_license();
                                is_pro.set(false);
                            },
                            "Deactivate license"
                        }
                    }
                } else {
                    div { class: "feature-card feature-card-locked",
                        div { class: "feature-card-top",
                            span { class: "feature-card-name", "Latency Analysis" }
                            span { class: "badge badge-orange feature-badge", "Pro" }
                        }
                        p { class: "feature-card-desc",
                            "Reconstruct full order chains from RFQ to fill."
                        }
                    }
                    div { class: "feature-card feature-card-locked",
                        div { class: "feature-card-top",
                            span { class: "feature-card-name", "Session Analysis" }
                            span { class: "badge badge-orange feature-badge", "Pro" }
                        }
                        p { class: "feature-card-desc",
                            "Fill quality scorecard & session health diagnostics."
                        }
                    }
                    div { class: "feature-card feature-card-locked",
                        div { class: "feature-card-top",
                            span { class: "feature-card-name", "FIX Validator" }
                            span { class: "badge badge-orange feature-badge", "Pro" }
                        }
                        p { class: "feature-card-desc",
                            "Validate messages against the FIX spec."
                        }
                    }
                    div { class: "license-upgrade-wrap",
                        button {
                            class: "btn-upgrade",
                            onclick: move |_| {
                                let _ = open::that(CHECKOUT_URL);
                            },
                            "Upgrade to Pro — $9.9/mo"
                        }
                        button {
                            class: "btn-activate-link",
                            onclick: move |_| {
                                license_input.set(String::new());
                                activate_error.set(None);
                                show_activate.set(true);
                            },
                            "I have a license key"
                        }
                    }
                    if *show_activate.read() {
                        div { class: "activate-dialog",
                            p { class: "activate-dialog-title", "Activate Pro" }
                            p { class: "activate-dialog-sub",
                                "Enter the license key from your purchase email."
                            }
                            input {
                                class: "activate-input",
                                r#type: "text",
                                placeholder: "XXXX-XXXX-XXXX-XXXX",
                                value: "{license_input}",
                                oninput: move |e| license_input.set(e.value()),
                            }
                            if let Some(err) = activate_error.read().clone() {
                                p { class: "activate-error", "{err}" }
                            }
                            div { class: "activate-dialog-actions",
                                button {
                                    class: "btn-activate-confirm",
                                    disabled: *activate_loading.read(),
                                    onclick: move |_| {
                                        let key = license_input.read().trim().to_string();
                                        if key.is_empty() { return; }
                                        activate_loading.set(true);
                                        activate_error.set(None);
                                        spawn(async move {
                                            match validate_license(&key).await {
                                                Ok(()) => {
                                                    save_license(&StoredLicense {
                                                        key: key.clone(),
                                                        instance_id: instance_name(),
                                                    });
                                                    is_pro.set(true);
                                                    show_activate.set(false);
                                                }
                                                Err(e) => { activate_error.set(Some(e)); }
                                            }
                                            activate_loading.set(false);
                                        });
                                    },
                                    if *activate_loading.read() { "Activating…" } else { "Activate" }
                                }
                                button {
                                    class: "btn-activate-cancel",
                                    onclick: move |_| show_activate.set(false),
                                    "Cancel"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
