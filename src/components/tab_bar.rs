use dioxus::prelude::*;
use dioxus::html::input_data::MouseButton as DxMouseButton;

use crate::tab::Tab;

#[derive(Clone, Copy)]
pub struct TabMenuPos {
    pub x:      i32,
    pub y:      i32,
    pub tab_id: u64,
}

/// Tab strip + new-tab button. Right-click on any tab opens the context menu
/// via `on_context_menu`. Compare and close actions move into that menu so the
/// strip itself stays clean.
#[component]
pub fn tab_bar(
    tabs:            Signal<Vec<Tab>>,
    active_id:       Signal<u64>,
    compare_id:      Signal<Option<u64>>,
    on_add:          EventHandler<()>,
    on_context_menu: EventHandler<TabMenuPos>,
) -> Element {
    let active   = *active_id.read();
    let compare  = *compare_id.read();
    let tab_list = tabs.read().clone();

    rsx! {
        div { class: "tab-strip",
            for (pos, t) in tab_list.iter().copied().enumerate() {
                {
                    let id        = t.id;
                    let label     = t.label.read().clone();
                    let is_active = id == active;
                    let is_cmp    = compare == Some(id);
                    let mut cls   = String::from("tab-chip");
                    if is_active { cls.push_str(" tab-chip-active"); }
                    if is_cmp    { cls.push_str(" tab-chip-compare"); }
                    // Append the keyboard shortcut hint to the tab tooltip for
                    // the first 9 tabs so power users discover ⌘1-9 jump.
                    let chip_title = if pos < 9 {
                        format!("{} (⌘{}  ·  right-click for actions)", label, pos + 1)
                    } else {
                        format!("{} (right-click for actions)", label)
                    };
                    rsx! {
                        div {
                            class: "{cls}",
                            title: "{chip_title}",
                            onclick: move |_| {
                                // Any click on a tab chip exits compare mode —
                                // user wants to focus on that tab, not keep an
                                // unrelated pane open. Done synchronously here
                                // so the very next render reflects the new state.
                                if compare_id.peek().is_some() {
                                    compare_id.set(None);
                                }
                                if *active_id.peek() != id {
                                    active_id.set(id);
                                }
                            },
                            // Suppress browser default + fire host handler with absolute coords.
                            oncontextmenu: move |e: MouseEvent| {
                                e.prevent_default();
                                let coords = e.client_coordinates();
                                on_context_menu.call(TabMenuPos {
                                    x: coords.x as i32,
                                    y: coords.y as i32,
                                    tab_id: id,
                                });
                            },
                            // Middle-click to close.
                            onmousedown: move |e: MouseEvent| {
                                if e.trigger_button() == Some(DxMouseButton::Auxiliary)
                                    && tabs.read().len() > 1
                                {
                                    let was_active  = *active_id.read() == id;
                                    let was_compare = *compare_id.read() == Some(id);
                                    tabs.with_mut(|v| v.retain(|t| t.id != id));
                                    if was_compare { compare_id.set(None); }
                                    if was_active {
                                        if let Some(first) = tabs.read().first().map(|t| t.id) {
                                            active_id.set(first);
                                            if *compare_id.read() == Some(first) { compare_id.set(None); }
                                        }
                                    }
                                }
                            },
                            if is_cmp {
                                span {
                                    class: "tab-chip-cmp-badge",
                                    title: "Comparing with active tab",
                                    "B"
                                }
                            }
                            span { class: "tab-chip-label", title: "{label}", "{label}" }
                            if tabs.read().len() > 1 {
                                button {
                                    class: "tab-chip-close",
                                    title: "Close tab (⌘W · middle-click)",
                                    onclick: move |evt: MouseEvent| {
                                        evt.stop_propagation();
                                        let closing_active  = *active_id.read() == id;
                                        let closing_compare = *compare_id.read() == Some(id);
                                        tabs.with_mut(|v| v.retain(|t| t.id != id));
                                        if closing_compare { compare_id.set(None); }
                                        if closing_active {
                                            if let Some(first) = tabs.read().first().map(|t| t.id) {
                                                active_id.set(first);
                                                if *compare_id.read() == Some(first) { compare_id.set(None); }
                                            }
                                        }
                                    },
                                    "×"
                                }
                            }
                        }
                    }
                }
            }
            button {
                class: "tab-add",
                title: "New tab (⌘T)",
                onclick: move |_| on_add.call(()),
                "+"
            }
        }
    }
}
