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
                        // Only show banner if we have multiple files (folder load) —
                        // single-file name is already in the tab chip.
                        rsx! {
                            if file_count > 0 {
                                div { class: "fix-file-banner",
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
                                if expanded {
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
