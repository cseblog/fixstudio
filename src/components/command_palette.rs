use dioxus::prelude::*;

/// A single command palette entry. `label` is shown to the user; `hint` is the
/// optional secondary text (file path, shortcut, etc.).
#[derive(Clone, PartialEq)]
pub struct CommandItem {
    pub id:    String,
    pub label: String,
    pub hint:  String,
}

/// Modal command palette (Cmd+K). Renders only when `open` is true. Fires
/// `on_pick(id)` when the user activates an item via click or Enter.
#[component]
pub fn command_palette(
    open:       Signal<bool>,
    items:      Vec<CommandItem>,
    on_pick:    EventHandler<String>,
) -> Element {
    let mut query    = use_signal(String::new);
    let mut selected = use_signal(|| 0usize);

    use_effect(move || {
        if *open.read() {
            query.set(String::new());
            selected.set(0);
        }
    });

    if !*open.read() { return rsx!{}; }

    let q = query.read().to_lowercase();
    let filtered: Vec<&CommandItem> = items.iter()
        .filter(|it| {
            if q.is_empty() { return true; }
            it.label.to_lowercase().contains(&q) || it.hint.to_lowercase().contains(&q)
        })
        .collect();
    let sel = (*selected.read()).min(filtered.len().saturating_sub(1));

    rsx! {
        div {
            class: "palette-overlay",
            onclick: move |_| open.set(false),
            div {
                class: "palette-modal",
                onclick: move |e| e.stop_propagation(),
                input {
                    class: "palette-input",
                    autofocus: true,
                    placeholder: "Type a command…",
                    value: "{query.read()}",
                    oninput: move |e| { query.set(e.value()); selected.set(0); },
                    onkeydown: {
                        let filtered_len = filtered.len();
                        let ids: Vec<String> = filtered.iter().map(|c| c.id.clone()).collect();
                        move |e: KeyboardEvent| {
                            match e.key() {
                                Key::Escape => open.set(false),
                                Key::ArrowDown => {
                                    let cur = *selected.read();
                                    selected.set((cur + 1).min(filtered_len.saturating_sub(1)));
                                }
                                Key::ArrowUp => {
                                    let cur = *selected.read();
                                    selected.set(cur.saturating_sub(1));
                                }
                                Key::Enter => {
                                    if let Some(id) = ids.get(*selected.read()) {
                                        let pick = id.clone();
                                        open.set(false);
                                        on_pick.call(pick);
                                    }
                                }
                                _ => {}
                            }
                        }
                    },
                }
                div { class: "palette-list",
                    for (i, it) in filtered.iter().enumerate() {
                        {
                            let id = it.id.clone();
                            let cls = if i == sel { "palette-item palette-item-active" } else { "palette-item" };
                            rsx! {
                                div {
                                    class: "{cls}",
                                    onclick: move |_| {
                                        let pick = id.clone();
                                        open.set(false);
                                        on_pick.call(pick);
                                    },
                                    onmouseenter: move |_| selected.set(i),
                                    span { class: "palette-item-label", "{it.label}" }
                                    if !it.hint.is_empty() {
                                        span { class: "palette-item-hint", "{it.hint}" }
                                    }
                                }
                            }
                        }
                    }
                    if filtered.is_empty() {
                        div { class: "palette-empty", "No commands match." }
                    }
                }
            }
        }
    }
}
