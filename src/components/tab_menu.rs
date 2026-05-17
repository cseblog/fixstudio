use dioxus::prelude::*;

/// Right-click context menu for a tab. Rendered at fixed position with the
/// tab ID baked into the action callbacks.
#[component]
pub fn tab_menu(
    x:               i32,
    y:               i32,
    tab_id:          u64,
    can_compare:     bool,
    is_compare_pair: bool,
    can_close:       bool,
    on_close:        EventHandler<()>,
    on_compare:      EventHandler<u64>,
    on_stop_compare: EventHandler<()>,
    on_duplicate:    EventHandler<u64>,
    on_close_tab:    EventHandler<u64>,
    on_close_others: EventHandler<u64>,
) -> Element {
    rsx! {
        div {
            class: "tab-menu-overlay",
            onclick: move |_| on_close.call(()),
            div {
                class: "tab-menu",
                style: "left: {x}px; top: {y}px;",
                onclick: move |e| e.stop_propagation(),

                if is_compare_pair {
                    div {
                        class: "tab-menu-item",
                        onclick: move |_| { on_stop_compare.call(()); on_close.call(()); },
                        "Stop comparing"
                    }
                } else if can_compare {
                    div {
                        class: "tab-menu-item",
                        onclick: move |_| { on_compare.call(tab_id); on_close.call(()); },
                        "Compare with active tab"
                    }
                }

                div { class: "tab-menu-sep" }

                div {
                    class: "tab-menu-item",
                    onclick: move |_| { on_duplicate.call(tab_id); on_close.call(()); },
                    "Duplicate tab"
                }
                div {
                    class: if can_close { "tab-menu-item" } else { "tab-menu-item tab-menu-item-disabled" },
                    onclick: move |_| if can_close { on_close_tab.call(tab_id); on_close.call(()); },
                    "Close tab"
                }
                div {
                    class: if can_close { "tab-menu-item" } else { "tab-menu-item tab-menu-item-disabled" },
                    onclick: move |_| if can_close { on_close_others.call(tab_id); on_close.call(()); },
                    "Close others"
                }
            }
        }
    }
}
