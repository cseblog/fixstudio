use std::collections::HashSet;

use dioxus::prelude::*;

use crate::components::detail::detail_panel;
use crate::components::hero::Hero;
use crate::components::lifecycle::lifecycle_panel;
use crate::components::overview::overview_panel;
use crate::components::timeline::timeline_panel;
use crate::components::validator_view::validator_panel;
use crate::model::FixMessage;
use crate::recents::RecentEntry;
use crate::tab::{message_key, Tab};
use crate::types::ViewMode;

#[component]
pub fn tab_view(
    tab: Tab,
    detail_visible: Signal<bool>,
    timeline_visible: Signal<bool>,
    recent_files: Vec<RecentEntry>,
    #[props(default)]
    compare_messages: Option<Signal<Vec<FixMessage>>>,
    #[props(default = false)]
    is_compare_pane: bool,
    #[props(default = false)]
    hide_view_tabs: bool,
    on_input:        EventHandler<String>,
    on_load_file:    EventHandler<()>,
    on_load_folder:  EventHandler<()>,
    on_load_sample:  EventHandler<String>,
    on_reload:       EventHandler<()>,
    on_open_recent:  EventHandler<String>,
    on_parse:        EventHandler<()>,
) -> Element {
    let Tab {
        input,
        messages,
        selected_idx,
        skip_heartbeats,
        skip_common,
        parse_stats,
        file_name,
        loaded_files,
        mut show_file_list,
        mut view_mode,
        loading,
        f_time,
        f_time_op,
        f_sender,
        f_target,
        f_msg,
        f_clord,
        f_detail,
        timeline_filters_open,
        display_limit,
        detail_view,
        detail_filter,
        detail_filter_open,
        validator_tab_kind,
        validator_raw_input,
        validator_filter,
        validator_report,
        validator_parsed_fields,
        validator_batch_reports,
        validator_batch_total,
        validator_batch_validating,
        validator_batch_signature,
        validator_cancel,
        lifecycle_chains,
        lifecycle_signature,
        lifecycle_computing,
        lifecycle_cancel,
        lifecycle_filter_id,
        file_path,
        mut file_auto_watch,
        ..
    } = tab;

    // Whenever the user navigates away from a heavy view (Validator / Latency),
    // abort any in-flight background job so it does not keep spinning on a
    // panel the user can't see.
    {
        let tab_for_cancel = tab;
        use_effect(move || {
            let mode = view_mode.read().clone();
            if mode != ViewMode::Validator { tab_for_cancel.cancel_validator(); }
            if mode != ViewMode::Lifecycle { tab_for_cancel.cancel_lifecycle(); }
        });
    }

    let compare_keys: Option<ReadSignal<HashSet<String>>> = compare_messages.map(|cmsgs| {
        use_memo(move || {
            cmsgs.read().iter().filter_map(message_key).collect::<HashSet<_>>()
        }).into()
    });

    let in_lifecycle = *view_mode.read() == ViewMode::Lifecycle;
    let in_overview  = *view_mode.read() == ViewMode::Overview;
    let in_validator = *view_mode.read() == ViewMode::Validator;
    let has_messages = !messages.read().is_empty();
    let show_hero    = messages.read().is_empty()
        && file_name.read().is_none()
        && !*loading.read();

    let sel        = *selected_idx.read();
    let detail_msg = sel.and_then(|i| messages.read().get(i).cloned());

    rsx! {
        div { class: if is_compare_pane { "tab-pane tab-pane-compare" } else { "tab-pane" },

            if show_hero {
                Hero {
                    on_load_file:   on_load_file,
                    on_load_folder: on_load_folder,
                    on_load_sample: on_load_sample,
                    on_open_recent: on_open_recent,
                    recent_files: recent_files.iter()
                        .map(|r| (r.path.clone(), r.name.clone()))
                        .collect(),
                }
                div { class: "input-with-action",
                    textarea {
                        class: "fix-input fix-input-hero",
                        placeholder: "…or paste FIX messages here",
                        value: "{input.read()}",
                        oninput: move |evt| on_input.call(evt.value()),
                    }
                    button {
                        class: "btn btn-process input-parse-btn",
                        disabled: input.read().is_empty(),
                        onclick: move |_| on_parse.call(()),
                        "Parse  ⌘↩"
                    }
                }
            } else {
                if *loading.read() {
                    div { class: "fix-loading", "Loading file and parsing messages…" }
                } else if has_messages && file_name.read().is_none() {
                    // Pasted FIX or sample: keep raw text visible (and editable) in a
                    // compact textarea above the timeline so the user can see and tweak
                    // the source. Press ⌘↩ to reparse.
                    div { class: "fix-file-banner fix-file-banner-pasted",
                        span { class: "fix-file-icon", "✏" }
                        span { class: "fix-file-name", "Pasted FIX" }
                        span { class: "fix-file-meta", "{messages.read().len()} messages · ⌘↩ to reparse" }
                    }
                    div { class: "input-with-action input-with-action-compact",
                        textarea {
                            class: "fix-input fix-input-compact",
                            placeholder: "Paste FIX messages here",
                            value: "{input.read()}",
                            oninput: move |evt| on_input.call(evt.value()),
                        }
                        button {
                            class: "btn btn-process input-parse-btn",
                            disabled: input.read().is_empty(),
                            onclick: move |_| on_parse.call(()),
                            "Reparse  ⌘↩"
                        }
                    }
                } else if let Some(_name) = file_name.read().clone() {
                    {
                        let files      = loaded_files.read();
                        let file_count = files.len();
                        let expanded   = *show_file_list.read();
                        let has_path   = file_path.read().is_some();
                        let auto_on    = *file_auto_watch.read();
                        rsx! {
                            // Single-file or folder load: a thin banner with
                            // Reload + Auto-watch controls so the user can
                            // pull in on-disk changes without re-picking
                            // through the file dialog.
                            if has_path || file_count > 0 {
                                div { class: "fix-file-banner",
                                    if has_path {
                                        button {
                                            class: "fix-file-toggle",
                                            title: "Re-read this file from disk and re-parse",
                                            onclick: move |_| on_reload.call(()),
                                            "↻ Reload"
                                        }
                                        button {
                                            class: if auto_on { "fix-file-toggle fix-file-toggle-on" } else { "fix-file-toggle" },
                                            title: "Watch this file and auto-reload when it changes (polls every 1.5s)",
                                            onclick: move |_| {
                                                let v = !*file_auto_watch.peek();
                                                file_auto_watch.set(v);
                                            },
                                            if auto_on { "● Auto-reload on" } else { "○ Auto-reload" }
                                        }
                                    }
                                    if file_count > 0 {
                                        button {
                                            class: "fix-file-toggle",
                                            onclick: move |_| {
                                                let cur = *show_file_list.read();
                                                show_file_list.set(!cur);
                                            },
                                            if expanded {
                                                "▾ {file_count} files"
                                            } else {
                                                "▸ {file_count} files"
                                            }
                                        }
                                    }
                                }
                                if expanded && file_count > 0 {
                                    div { class: "fix-file-list",
                                        for f in files.iter() {
                                            div { class: "fix-file-list-item", "{f}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if !has_messages && (!in_validator || !input.read().is_empty()) {
                    div { class: "input-with-action",
                        textarea {
                            class: "fix-input",
                            placeholder: "Paste FIX messages here",
                            value: "{input.read()}",
                            oninput: move |evt| on_input.call(evt.value()),
                        }
                        button {
                            class: "btn btn-process input-parse-btn",
                            disabled: input.read().is_empty(),
                            onclick: move |_| on_parse.call(()),
                            "Parse  ⌘↩"
                        }
                    }
                }

                // Anomaly banner — silent on healthy logs; loud on bursts.
                // Sits above the view-tabs so it's the first thing the user
                // sees on a new file load.
                if has_messages && !is_compare_pane {
                    {
                        let anomalies = use_memo(move || {
                            crate::anomaly::scan(&messages.read())
                        });
                        let list = anomalies.read().clone();
                        if !list.is_empty() {
                            rsx! {
                                div { class: "anomaly-banner",
                                    for a in list.iter() {
                                        {
                                            let (icon, label, sev) = render_anomaly(a);
                                            rsx! {
                                                span {
                                                    class: if sev == "crit" { "anom-chip anom-crit" } else { "anom-chip anom-warn" },
                                                    span { class: "anom-icon", "{icon}" }
                                                    span { class: "anom-text", "{label}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    }
                }

                if (has_messages || in_validator) && !hide_view_tabs {
                    div { class: "panel-tabs",
                        button {
                            class: if !in_lifecycle && !in_overview && !in_validator {
                                "panel-tab panel-tab-active"
                            } else { "panel-tab" },
                            onclick: move |_| view_mode.set(ViewMode::Timeline),
                            "Timeline"
                        }
                        button {
                            class: if in_lifecycle { "panel-tab panel-tab-active" } else { "panel-tab" },
                            onclick: move |_| view_mode.set(ViewMode::Lifecycle),
                            "Latency"
                        }
                        button {
                            class: if in_overview { "panel-tab panel-tab-active" } else { "panel-tab" },
                            onclick: move |_| view_mode.set(ViewMode::Overview),
                            "Session"
                        }
                        button {
                            class: if in_validator { "panel-tab panel-tab-active" } else { "panel-tab" },
                            onclick: move |_| view_mode.set(ViewMode::Validator),
                            "Validator"
                        }
                    }
                }

                if in_validator {
                    validator_panel {
                        messages: messages,
                        tab_kind: validator_tab_kind,
                        raw_input: validator_raw_input,
                        filter_text: validator_filter,
                        report: validator_report,
                        parsed_fields: validator_parsed_fields,
                        batch_reports: validator_batch_reports,
                        batch_total: validator_batch_total,
                        validating: validator_batch_validating,
                        batch_signature: validator_batch_signature,
                        cancel: validator_cancel,
                    }
                } else if in_overview {
                    overview_panel { messages: messages }
                } else if in_lifecycle {
                    lifecycle_panel {
                        messages: messages,
                        selected_idx: selected_idx,
                        chains_state: lifecycle_chains,
                        chains_signature: lifecycle_signature,
                        chains_computing: lifecycle_computing,
                        cancel: lifecycle_cancel,
                        filter_id: lifecycle_filter_id,
                    }
                } else {
                    {
                        let show_tl = *timeline_visible.read();
                        let show_dt = *detail_visible.read();
                        let mut cls = String::from("panels");
                        if !show_dt { cls.push_str(" panels-no-detail"); }
                        if !show_tl { cls.push_str(" panels-no-timeline"); }
                        rsx! {
                            if !show_tl && !show_dt {
                                div { class: "empty-state empty-state-ghost",
                                    "Both Timeline and Detail are hidden. Press ⌘L to show Timeline · ⌘B to show Detail."
                                }
                            } else {
                                div { class: "{cls}",
                                    if show_tl {
                                        timeline_panel {
                                            messages: messages,
                                            selected_idx: selected_idx,
                                            skip_heartbeats: skip_heartbeats,
                                            parse_stats: parse_stats,
                                            f_time: f_time,
                                            f_time_op: f_time_op,
                                            f_sender: f_sender,
                                            f_target: f_target,
                                            f_msg: f_msg,
                                            f_clord: f_clord,
                                            f_detail: f_detail,
                                            display_limit: display_limit,
                                            filters_open: timeline_filters_open,
                                            compare_keys: compare_keys,
                                            on_jump_to_chain: move |id: String| {
                                                // Pre-fill the latency view's chain filter, then
                                                // switch this tab's view_mode so the user lands
                                                // directly on the matching chain.
                                                lifecycle_filter_id.clone().set(id);
                                                view_mode.set(ViewMode::Lifecycle);
                                            },
                                        }
                                    }
                                    if show_dt {
                                        detail_panel {
                                            detail_msg: detail_msg,
                                            skip_common: skip_common,
                                            selected_idx: selected_idx,
                                            view_kind: detail_view,
                                            table_filter: detail_filter,
                                            filter_open: detail_filter_open,
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Render one anomaly as (icon, plain-text label, severity-class).
/// Kept out of rsx! so the banner template stays scannable.
fn render_anomaly(a: &crate::anomaly::Anomaly) -> (&'static str, String, &'static str) {
    use crate::anomaly::{Anomaly, Severity};
    let sev_str = |s: &Severity| if *s == Severity::Critical { "crit" } else { "warn" };
    match a {
        Anomaly::RejectBurst { count, window_secs, severity } => (
            "⚠",
            format!("Reject burst — {count} rejects in {window_secs}s"),
            sev_str(severity),
        ),
        Anomaly::SequenceGap { sender, target, gap, .. } => (
            "↯",
            format!("Sequence gap — {sender}→{target} skipped {gap} msgs"),
            sev_str(if *gap >= 10 { &Severity::Critical } else { &Severity::Warn }),
        ),
        Anomaly::LatencySpike { recent_p50_us, baseline_p50_us, multiple, severity } => (
            "🐌",
            format!(
                "Latency spike — recent p50 {}ms vs baseline {}ms ({:.1}×)",
                recent_p50_us   / 1_000,
                baseline_p50_us / 1_000,
                multiple,
            ),
            sev_str(severity),
        ),
    }
}
