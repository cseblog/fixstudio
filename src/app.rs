use std::mem;
use std::time::Instant;

use dioxus::prelude::*;
use dioxus::document::eval;

use crate::components::detail::detail_panel;
use crate::components::hero::Hero;
use crate::components::lifecycle::lifecycle_panel;
use crate::components::overview::overview_panel;
use crate::components::premium_panel::premium_panel;
use crate::components::timeline::timeline_panel;
use crate::components::validator_view::validator_panel;
use crate::loader::{pick_and_load_file, pick_and_load_folder};
use crate::model::FixMessage;
use crate::parser::parse_all;
use crate::sample::{sample_data, FIX_SPECS};
use crate::style::CSS;
use crate::types::{is_newer_version, UpdateStatus, ViewMode};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const VERSION_URL: &str     = "https://aifixparser.com/latest-version";
const DOWNLOAD_URL: &str    = "https://aifixparser.com/#download";
const GA_ID: &str           = "G-Y9J423BNZ0";

// JS for the persistent drag-to-resize handler on the right panel.
const RESIZE_JS: &str = r#"
(function() {
    if (window._rszCleanup) window._rszCleanup();
    var handle = document.querySelector('.resize-handle');
    if (!handle) return;

    function ondown(e) {
        if (e.target.closest && e.target.closest('button')) return;
        if (e.target.tagName === 'BUTTON') return;
        var panel = document.getElementById('premium-panel-main');
        if (!panel) return;
        var sw = parseFloat(panel.style.width);
        if (!sw || sw < 10) {
            var styleAttr = panel.getAttribute('style') || '';
            if (styleAttr.indexOf('flex: 1') >= 0) return;
            sw = panel.getBoundingClientRect().width;
        }
        if (!sw || sw < 10) return;
        document.body.style.userSelect = 'none';
        document.body.style.webkitUserSelect = 'none';
        var sx = e.clientX, go = false;
        function onmove(e2) {
            var dx = sx - e2.clientX;
            if (!go) {
                if (Math.abs(dx) < 4) return;
                go = true;
                document.body.style.cursor = 'col-resize';
            }
            var nw = Math.max(200, Math.min(window.innerWidth - 50, sw + dx));
            panel.style.width = nw + 'px';
        }
        function onup() {
            document.removeEventListener('mousemove', onmove);
            document.removeEventListener('mouseup', onup);
            document.body.style.cursor = '';
            document.body.style.userSelect = '';
            document.body.style.webkitUserSelect = '';
            if (go) window.dioxus.send('w:' + parseFloat(panel.style.width));
        }
        document.addEventListener('mousemove', onmove);
        document.addEventListener('mouseup', onup);
    }

    handle.addEventListener('mousedown', ondown);
    window._rszCleanup = function() {
        handle.removeEventListener('mousedown', ondown);
        window._rszCleanup = null;
    };
})();
"#;

fn offload_replace(signal: &mut Signal<Vec<FixMessage>>, new_data: Vec<FixMessage>) {
    let old = mem::replace(&mut *signal.write(), new_data);
    if !old.is_empty() {
        std::thread::spawn(move || drop(old));
    }
}

/// Root application component.
pub fn app() -> Element {
    // ── Core state ──
    let mut input                                 = use_signal(String::new);
    let mut messages: Signal<Vec<FixMessage>>     = use_signal(Vec::new);
    let mut selected_idx: Signal<Option<usize>>   = use_signal(|| None);
    let skip_heartbeats                           = use_signal(|| true);
    let skip_common                               = use_signal(|| false);
    let mut parse_stats: Signal<Option<(usize, u64)>> = use_signal(|| None);
    let loading                                   = use_signal(|| false);
    let mut file_name: Signal<Option<String>>     = use_signal(|| None);
    let mut loaded_files: Signal<Vec<String>>     = use_signal(Vec::new);
    let mut show_file_list                        = use_signal(|| false);
    let mut update_status: Signal<UpdateStatus>   = use_signal(|| UpdateStatus::Idle);
    let mut view_mode                             = use_signal(|| ViewMode::Timeline);

    // ── Panel layout state ──
    let right_panel_width: Signal<f64> = use_signal(|| 200.0_f64);
    let left_panel_collapsed           = use_signal(|| false);
    let mut right_panel_collapsed      = use_signal(|| false);

    // Initialise right panel to half the window width via JS.
    use_effect(move || {
        let mut rpw = right_panel_width;
        spawn(async move {
            if let Ok(v) = eval("window.innerWidth").await {
                if let Some(w) = v.as_f64() {
                    rpw.set((w / 2.0).max(200.0));
                }
            }
        });
    });

    // Persistent drag-to-resize handler — set up once on mount.
    use_effect(move || {
        let mut rpw = right_panel_width;
        spawn(async move {
            let mut ev = eval(RESIZE_JS);
            loop {
                match ev.recv::<String>().await {
                    Ok(s) if s.starts_with("w:") => {
                        if let Ok(w) = s[2..].parse::<f64>() { rpw.set(w); }
                    }
                    _ => break,
                }
            }
        });
    });

    // ── Actions ──
    let mut process = move || {
        let t      = Instant::now();
        let parsed = parse_all(&input.read());
        let ms     = t.elapsed().as_micros() as u64;
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
        loaded_files.set(Vec::new());
        show_file_list.set(false);
        view_mode.set(ViewMode::Timeline);
    };

    let mut load_sample = move |spec: &str| {
        let s      = sample_data(spec);
        let t      = Instant::now();
        let parsed = parse_all(&s);
        let ms     = t.elapsed().as_micros() as u64;
        let count  = parsed.len();
        parse_stats.set(Some((count, ms)));
        input.set(s);
        offload_replace(&mut messages, parsed);
        selected_idx.set(None);
        file_name.set(None);
        eval(&format!(
            "window.gtag && window.gtag('event', 'sample_loaded', \
             {{ sample: '{spec}', message_count: {count}, parse_us: {ms} }});"
        ));
    };

    let load_file = move || {
        let mut messages     = messages;
        let mut selected_idx = selected_idx;
        let mut parse_stats  = parse_stats;
        let mut loading      = loading;
        let mut file_name    = file_name;
        spawn(async move {
            loading.set(true);
            if let Some(r) = pick_and_load_file().await {
                let count     = r.messages.len();
                let ms        = r.parse_us as u64;
                let delimiter = if r.is_soh { "soh" } else { "pipe" };
                parse_stats.set(Some((count, ms)));
                offload_replace(&mut messages, r.messages);
                selected_idx.set(None);
                file_name.set(Some(r.name));
                eval(&format!(
                    "window.gtag && window.gtag('event', 'file_parsed', \
                     {{ message_count: {count}, parse_us: {ms}, delimiter: '{delimiter}' }});"
                ));
            }
            loading.set(false);
        });
    };

    let load_folder = move || {
        let mut messages     = messages;
        let mut selected_idx = selected_idx;
        let mut parse_stats  = parse_stats;
        let mut loading      = loading;
        let mut file_name    = file_name;
        let mut loaded_files = loaded_files;
        spawn(async move {
            loading.set(true);
            if let Some(r) = pick_and_load_folder().await {
                let count = r.messages.len();
                let ms    = r.parse_us as u64;
                parse_stats.set(Some((count, ms)));
                offload_replace(&mut messages, r.messages);
                selected_idx.set(None);
                file_name.set(Some(r.folder_name));
                loaded_files.set(r.file_names);
                eval(&format!(
                    "window.gtag && window.gtag('event', 'folder_parsed', \
                     {{ message_count: {count}, parse_us: {ms} }});"
                ));
            }
            loading.set(false);
        });
    };

    // ── On-mount effects ──

    // Inject GA4 script and fire app_open event.
    use_effect(move || {
        let os = std::env::consts::OS;
        eval(&format!(
            r#"(function(id) {{
                if (window._ga_inited) return;
                window._ga_inited = true;
                var s = document.createElement('script');
                s.async = true;
                s.src = 'https://www.googletagmanager.com/gtag/js?id=' + id;
                document.head.appendChild(s);
                s.onload = function() {{
                    window.dataLayer = window.dataLayer || [];
                    function gtag(){{ window.dataLayer.push(arguments); }}
                    window.gtag = gtag;
                    gtag('js', new Date());
                    gtag('config', id, {{ send_page_view: false }});
                    var _ga_start = Date.now();
                    gtag('event', 'app_open', {{ app_version: '{CURRENT_VERSION}', platform: '{os}' }});
                    window.addEventListener('beforeunload', function() {{
                        var sec = Math.round((Date.now() - _ga_start) / 1000);
                        gtag('event', 'session_end', {{
                            session_duration_sec: sec,
                            app_version: '{CURRENT_VERSION}',
                            platform: '{os}',
                            transport_type: 'beacon'
                        }});
                    }});
                }};
            }})('{GA_ID}');"#
        ));
    });

    // Auto-check for updates via JS fetch.
    use_effect(move || {
        update_status.set(UpdateStatus::Checking);
        spawn(async move {
            let mut ev = eval(&format!(
                r#"(async () => {{
                    try {{
                        const r = await fetch('{VERSION_URL}',
                            {{ signal: AbortSignal.timeout(6000) }});
                        window.dioxus.send((await r.text()).trim());
                    }} catch(e) {{
                        window.dioxus.send('');
                    }}
                }})();"#
            ));
            update_status.set(match ev.recv::<String>().await {
                Ok(v) if !v.is_empty() && is_newer_version(&v, CURRENT_VERSION) => {
                    UpdateStatus::Available(v)
                }
                Ok(v) if !v.is_empty() => UpdateStatus::UpToDate,
                _ => UpdateStatus::Idle,
            });
        });
    });

    // ── Derived state ──
    let sel         = *selected_idx.read();
    let detail_msg  = sel.and_then(|i| messages.read().get(i).cloned());
    let show_hero   = messages.read().is_empty()
        && file_name.read().is_none()
        && !*loading.read();
    let has_messages    = !messages.read().is_empty();
    let in_lifecycle    = *view_mode.read() == ViewMode::Lifecycle;
    let in_overview     = *view_mode.read() == ViewMode::Overview;
    let in_validator    = *view_mode.read() == ViewMode::Validator;
    let left_collapsed  = *left_panel_collapsed.read();
    let right_collapsed = *right_panel_collapsed.read();

    rsx! {
        style { {CSS} }
        div { class: "root",

            // ── Toolbar ──
            div { class: "toolbar",
                if file_name.read().is_none() {
                    button { class: "btn btn-process", onclick: move |_| process(), "Process" }
                }
                button { class: "btn btn-clear",   onclick: move |_| clear(),       "Clear" }
                button { class: "btn btn-load",    onclick: move |_| load_file(),   "Load file" }
                button { class: "btn btn-load",    onclick: move |_| load_folder(), "Load folder" }

                span { class: "sample-label", "Sample: " }
                {FIX_SPECS.iter().enumerate().map(|(i, spec)| {
                    let sep = if i > 0 {
                        rsx! { span { class: "sample-sep", " | " } }
                    } else {
                        rsx! { }
                    };
                    rsx! {
                        {sep}
                        button {
                            class: "btn btn-sample-inline",
                            onclick: move |_| load_sample(spec),
                            "{spec}"
                        }
                    }
                })}

                div { class: "toolbar-spacer" }

                {
                    let status = update_status.read().clone();
                    match status {
                        UpdateStatus::Checking => rsx! {
                            span { class: "update-checking", "Checking for updates…" }
                        },
                        UpdateStatus::Available(v) => rsx! {
                            button {
                                class: "btn btn-update-available",
                                onclick: move |_| {
                                    let url = DOWNLOAD_URL.to_string();
                                    std::thread::spawn(move || { let _ = open::that(url); });
                                },
                                "⬆ v{v} — Update now"
                            }
                        },
                        UpdateStatus::UpToDate => rsx! {
                            span { class: "update-ok", "✓ v{CURRENT_VERSION}" }
                        },
                        UpdateStatus::Idle => rsx! {
                            span { class: "update-version", "v{CURRENT_VERSION}" }
                        },
                    }
                }
            }

            // ── Two-panel body ──
            div { class: "app-body",

                // Left: main content
                div {
                    class: "app-main",
                    style: if left_collapsed {
                        "flex: none; width: 0; min-width: 0; overflow: hidden;"
                    } else {
                        "flex: 1; min-width: 0;"
                    },

                    if show_hero {
                        Hero {}
                    } else {
                        if *loading.read() {
                            div { class: "fix-loading", "Loading file and parsing messages…" }
                        } else if let Some(ref name) = *file_name.read() {
                            {
                                let files      = loaded_files.read();
                                let file_count = files.len();
                                let expanded   = *show_file_list.read();
                                rsx! {
                                    div { class: "fix-file-banner",
                                        span { class: "fix-file-icon", "📂" }
                                        span { class: "fix-file-name", "{name}" }
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
                                        span { class: "fix-file-hint", "— click Clear to reset" }
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
                        } else if !in_validator || !input.read().is_empty() {
                            textarea {
                                class: "fix-input",
                                placeholder: "Paste FIX messages here …",
                                value: "{input.read()}",
                                oninput: move |evt| input.set(evt.value()),
                            }
                        }

                        if has_messages || in_validator {
                            div { class: "panel-tabs",
                                button {
                                    class: if !in_lifecycle && !in_overview && !in_validator {
                                        "panel-tab panel-tab-active"
                                    } else {
                                        "panel-tab"
                                    },
                                    onclick: move |_| view_mode.set(ViewMode::Timeline),
                                    "Timeline"
                                }
                                button {
                                    class: if in_lifecycle {
                                        "panel-tab panel-tab-active"
                                    } else {
                                        "panel-tab"
                                    },
                                    onclick: move |_| view_mode.set(ViewMode::Lifecycle),
                                    "Latency Analysis"
                                }
                                button {
                                    class: if in_overview {
                                        "panel-tab panel-tab-active"
                                    } else {
                                        "panel-tab"
                                    },
                                    onclick: move |_| view_mode.set(ViewMode::Overview),
                                    "Session Analysis"
                                }
                                button {
                                    class: if in_validator {
                                        "panel-tab panel-tab-active"
                                    } else {
                                        "panel-tab"
                                    },
                                    onclick: move |_| view_mode.set(ViewMode::Validator),
                                    "FIX Validator"
                                }
                            }
                        }

                        if in_validator {
                            validator_panel { messages: messages }
                        } else if in_overview {
                            overview_panel { messages: messages }
                        } else if in_lifecycle {
                            lifecycle_panel {
                                messages: messages,
                                selected_idx: selected_idx,
                            }
                        } else {
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
                                    selected_idx: selected_idx,
                                }
                            }
                        }
                    }
                }

                // Resize handle — drag handled by RESIZE_JS effect above.
                div {
                    class: "resize-handle",
                    div { class: "resize-handle-bar" }
                    div { class: "collapse-panel-btns",
                        button {
                            class: "collapse-panel-btn",
                            title: if right_collapsed {
                                "Expand right panel"
                            } else {
                                "Collapse right panel"
                            },
                            onclick: move |_| {
                                let c = *right_panel_collapsed.read();
                                if c {
                                    right_panel_collapsed.set(false);
                                } else {
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
                                }
                            },
                            if right_collapsed { "▶" } else { "◀" }
                        }
                    }
                }

                // Right: premium panel component
                premium_panel {
                    has_messages: has_messages,
                    view_mode: view_mode,
                    right_panel_collapsed: right_panel_collapsed,
                    right_panel_width: right_panel_width,
                    left_collapsed: left_collapsed,
                }
            }
        }
    }
}
