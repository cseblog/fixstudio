use std::time::Instant;
use std::mem;

use dioxus::prelude::*;
use dioxus::document::eval;

use crate::components::detail::detail_panel;
use crate::components::lifecycle::lifecycle_panel;
use crate::components::overview::overview_panel;
use crate::components::timeline::timeline_panel;
use crate::components::validator_view::validator_panel;
use crate::model::FixMessage;
use crate::parser::{parse_all, parse_all_simd, parse_all_simd_bytes};
use crate::sample::{sample_data, FIX_SPECS};
use crate::style::CSS;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const VERSION_URL: &str = "https://aifixparser.com/latest-version";
const DOWNLOAD_URL: &str = "https://aifixparser.com/#download";
const GA_ID: &str = "G-Y9J423BNZ0";

#[derive(Clone, PartialEq)]
enum UpdateStatus {
    Idle,
    Checking,
    Available(String),
    UpToDate,
}

#[derive(Clone, PartialEq)]
enum ViewMode {
    Timeline,
    Lifecycle,
    Overview,
    Validator,
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse = |s: &str| -> [u32; 3] {
        let mut it = s.split('.').filter_map(|p| p.parse().ok());
        [it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0)]
    };
    parse(latest) > parse(current)
}

fn offload_replace(signal: &mut Signal<Vec<FixMessage>>, new_data: Vec<FixMessage>) {
    let old = mem::replace(&mut *signal.write(), new_data);
    if !old.is_empty() {
        std::thread::spawn(move || drop(old));
    }
}

/// Root application component.
pub fn app() -> Element {
    // ── Core state ──
    let mut input = use_signal(String::new);
    let mut messages: Signal<Vec<FixMessage>> = use_signal(Vec::new);
    let mut selected_idx: Signal<Option<usize>> = use_signal(|| None);
    let skip_heartbeats = use_signal(|| true);
    let skip_common = use_signal(|| false);
    let mut parse_stats: Signal<Option<(usize, u64)>> = use_signal(|| None);
    let loading = use_signal(|| false);
    let mut file_name: Signal<Option<String>> = use_signal(|| None);
    let mut loaded_files: Signal<Vec<String>> = use_signal(Vec::new);
    let mut show_file_list = use_signal(|| false);
    let mut update_status: Signal<UpdateStatus> = use_signal(|| UpdateStatus::Idle);
    let mut view_mode = use_signal(|| ViewMode::Timeline);

    // ── Panel layout state ──
    // right_panel_width controls premium-panel; app-main is flex:1 filling the rest
    let right_panel_width: Signal<f64> = use_signal(|| 200.0_f64);
    let left_panel_collapsed  = use_signal(|| false);
    let mut right_panel_collapsed = use_signal(|| false);

    // Initialise right panel to half the window width via JS
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

    // ── Persistent JS-native resize handler ──
    // Set up once on mount. Uses direct DOM manipulation for smooth 60fps dragging.
    // Checks e.target.closest('button') so collapse buttons are never treated as drag starts.
    use_effect(move || {
        let mut rpw = right_panel_width;
        spawn(async move {
            let mut ev = eval(r#"
(function() {
    if (window._rszCleanup) window._rszCleanup();
    var handle = document.querySelector('.resize-handle');
    if (!handle) return;

    function ondown(e) {
        // Let button clicks pass through untouched
        if (e.target.closest && e.target.closest('button')) return;
        if (e.target.tagName === 'BUTTON') return;

        var panel = document.getElementById('premium-panel-main');
        if (!panel) return;

        // Read current width from inline style; fall back to rendered width
        var sw = parseFloat(panel.style.width);
        if (!sw || sw < 10) {
            var styleAttr = panel.getAttribute('style') || '';
            if (styleAttr.indexOf('flex: 1') >= 0) return; // right fills screen (left collapsed)
            sw = panel.getBoundingClientRect().width;
        }
        if (!sw || sw < 10) return; // panel is collapsed/hidden

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
            panel.style.width = nw + 'px'; // direct DOM — no Rust roundtrip
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
            "#);
            // Receive final width once per drag (not per mousemove)
            loop {
                match ev.recv::<String>().await {
                    Ok(s) if s.starts_with("w:") => {
                        if let Ok(w) = s[2..].parse::<f64>() {
                            rpw.set(w);
                        }
                        // keep looping — handler stays active for future drags
                    }
                    _ => break,
                }
            }
        });
    });

    // ── Actions ──
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
        loaded_files.set(Vec::new());
        show_file_list.set(false);
        view_mode.set(ViewMode::Timeline);
    };

    let mut load_sample = move |spec: &str| {
        let s = sample_data(spec);
        let t = Instant::now();
        let parsed = parse_all(&s);
        let ms = t.elapsed().as_micros() as u64;
        parse_stats.set(Some((parsed.len(), ms)));
        let count = parsed.len();
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
        let mut messages     = messages.clone();
        let mut selected_idx = selected_idx.clone();
        let mut parse_stats  = parse_stats.clone();
        let mut loading      = loading.clone();
        let mut file_name    = file_name.clone();
        spawn(async move {
            loading.set(true);
            if let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("FIX log", &["txt", "log", "fix"])
                .add_filter("All files", &["*"])
                .pick_file()
                .await
            {
                let name = file.file_name();
                let path = file.path().to_owned();
                let t = Instant::now();
                let (parsed, is_soh) = match std::fs::File::open(&path)
                    .and_then(|f| unsafe { memmap2::Mmap::map(&f) })
                {
                    Ok(mmap) => {
                        let soh = mmap.iter().take(4096).any(|&b| b == 0x01);
                        let msgs = if soh {
                            parse_all_simd_bytes(&mmap)
                        } else {
                            let s = String::from_utf8_lossy(&mmap);
                            parse_all(&s)
                        };
                        (msgs, soh)
                    }
                    Err(_) => {
                        let bytes = file.read().await;
                        let soh = bytes.iter().take(4096).any(|&b| b == 0x01);
                        let s = String::from_utf8_lossy(&bytes);
                        let msgs = if soh { parse_all_simd(&s) } else { parse_all(&s) };
                        (msgs, soh)
                    }
                };
                let ms = t.elapsed().as_micros() as u64;
                let count = parsed.len();
                parse_stats.set(Some((count, ms)));
                offload_replace(&mut messages, parsed);
                selected_idx.set(None);
                file_name.set(Some(name));
                let delimiter = if is_soh { "soh" } else { "pipe" };
                eval(&format!(
                    "window.gtag && window.gtag('event', 'file_parsed', \
                     {{ message_count: {count}, parse_us: {ms}, delimiter: '{delimiter}' }});"
                ));
            }
            loading.set(false);
        });
    };

    let load_folder = move || {
        let mut messages     = messages.clone();
        let mut selected_idx = selected_idx.clone();
        let mut parse_stats  = parse_stats.clone();
        let mut loading      = loading.clone();
        let mut file_name    = file_name.clone();
        let mut loaded_files = loaded_files.clone();
        spawn(async move {
            loading.set(true);
            if let Some(folder) = rfd::AsyncFileDialog::new().pick_folder().await {
                let root = folder.path().to_owned();
                let t = Instant::now();
                // Guard against pathological trees (e.g. symlink cycles).
                const MAX_DIRS: usize = 4_096;
                let fix_exts = ["txt", "log", "fix"];
                let mut all_msgs: Vec<FixMessage> = Vec::new();
                let mut file_names: Vec<String>   = Vec::new();
                let mut dir_stack  = vec![root.clone()];
                let mut dirs_seen  = 0_usize;
                while let Some(dir) = dir_stack.pop() {
                    assert!(dirs_seen <= MAX_DIRS);
                    dirs_seen += 1;
                    if dirs_seen > MAX_DIRS { break; }
                    if let Ok(rd) = std::fs::read_dir(&dir) {
                        for entry in rd.flatten() {
                            let p = entry.path();
                            if p.is_dir() {
                                dir_stack.push(p);
                            } else if p.extension()
                                .and_then(|e| e.to_str())
                                .map(|e| fix_exts.contains(&e))
                                .unwrap_or(false)
                            {
                                let msgs = match std::fs::File::open(&p)
                                    .and_then(|f| unsafe { memmap2::Mmap::map(&f) })
                                {
                                    Ok(mmap) => {
                                        // Only process files that contain the FIX BeginString tag.
                                        // This filters out logs, configs, and other text files that
                                        // happen to have a matching extension.
                                        let has_fix = mmap.windows(5)
                                            .any(|w| w == b"8=FIX");
                                        if !has_fix { continue; }
                                        let soh = mmap.iter().take(4096).any(|&b| b == 0x01);
                                        if soh { parse_all_simd_bytes(&mmap) }
                                        else {
                                            let s = String::from_utf8_lossy(&mmap);
                                            parse_all(&s)
                                        }
                                    }
                                    Err(_) => continue,
                                };
                                if msgs.is_empty() { continue; }
                                // Relative path makes the list readable regardless of mount point.
                                let rel = p.strip_prefix(&root)
                                    .unwrap_or(&p)
                                    .to_string_lossy()
                                    .into_owned();
                                file_names.push(format!("{rel} ({} msgs)", msgs.len()));
                                all_msgs.extend(msgs);
                            }
                        }
                    }
                }
                file_names.sort();
                let ms = t.elapsed().as_micros() as u64;
                let count = all_msgs.len();
                parse_stats.set(Some((count, ms)));
                offload_replace(&mut messages, all_msgs);
                selected_idx.set(None);
                let folder_name = root.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("folder")
                    .to_string();
                file_name.set(Some(folder_name));
                loaded_files.set(file_names);
                eval(&format!(
                    "window.gtag && window.gtag('event', 'folder_parsed', \
                     {{ message_count: {count}, parse_us: {ms} }});"
                ));
            }
            loading.set(false);
        });
    };

    // ── On-mount effects ──

    // 1. Inject GA4 + fire app_open
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

    // 2. Auto-check for updates
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

    // 3. Hero counter animation
    use_effect(move || {
        let visible = messages.read().is_empty()
            && file_name.read().is_none()
            && !*loading.read();
        if visible {
            eval(r#"
                (function() {
                    function animCounter(id, target, dur) {
                        var el = document.getElementById(id);
                        if (!el) return;
                        var t0 = performance.now();
                        (function tick() {
                            var p = Math.min((performance.now() - t0) / dur, 1);
                            var v = Math.round((1 - Math.pow(1 - p, 3)) * target);
                            el.textContent = (p >= 1 ? target : v).toLocaleString('en-US');
                            if (p < 1) requestAnimationFrame(tick);
                        })();
                    }
                    setTimeout(function() {
                        animCounter('hero-msg-count',  1000000, 1600);
                        animCounter('hero-parse-ms',   150,     1200);
                        animCounter('hero-throughput', 631,     1000);
                    }, 150);
                })();
            "#);
        }
    });

    // ── Derived state ──
    let sel = *selected_idx.read();
    let detail_msg: Option<FixMessage> = sel.and_then(|i| messages.read().get(i).cloned());
    let show_hero = messages.read().is_empty()
        && file_name.read().is_none()
        && !*loading.read();
    let has_messages = !messages.read().is_empty();
    let pro = true;
    let in_lifecycle  = *view_mode.read() == ViewMode::Lifecycle;
    let in_overview   = *view_mode.read() == ViewMode::Overview;
    let in_validator  = *view_mode.read() == ViewMode::Validator;
    let left_collapsed  = *left_panel_collapsed.read();
    let right_collapsed = *right_panel_collapsed.read();
    let right_w = *right_panel_width.read() as u32;

    rsx! {
        style { {CSS} }
        div { class: "root",

            // ── Toolbar ──
            div { class: "toolbar",
                if file_name.read().is_none() {
                    button { class: "btn btn-process", onclick: move |_| process(), "Process" }
                }
                button { class: "btn btn-clear", onclick: move |_| clear(), "Clear" }
                button { class: "btn btn-load", onclick: move |_| load_file(), "Load file" }
                button { class: "btn btn-load", onclick: move |_| load_folder(), "Load folder" }

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

                div { class: "toolbar-spacer" }

                // Update indicator
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

                // ── Left: main content ──
                div {
                    class: "app-main",
                    style: if left_collapsed { "flex: none; width: 0; min-width: 0; overflow: hidden;" } else { "flex: 1; min-width: 0;" },

                    if show_hero {
                        // ── Hero landing ──
                        div { class: "hero",
                            div { class: "hero-title",
                                span { class: "hero-icon", "⚡" }
                                h1 { "AiFIXParser.com" }
                                p { "Lightning fast FIX protocol parser & inspector" }
                            }
                            div { class: "hero-stats",
                                div { class: "hero-stat hero-stat-a",
                                    div { class: "hero-stat-value", id: "hero-msg-count", "0" }
                                    div { class: "hero-stat-unit", "messages" }
                                    div { class: "hero-stat-label", "parsed in one shot" }
                                }
                                div { class: "hero-stat hero-stat-featured",
                                    div { class: "hero-stat-value",
                                        span { id: "hero-parse-ms", "0" }
                                        span { class: "hero-stat-suffix", "ms" }
                                    }
                                    div { class: "hero-stat-unit", "parse time" }
                                    div { class: "hero-stat-label", "for 1,000,000 messages" }
                                }
                                div { class: "hero-stat hero-stat-b",
                                    div { class: "hero-stat-value",
                                        span { id: "hero-throughput", "0" }
                                        span { class: "hero-stat-suffix", " MiB/s" }
                                    }
                                    div { class: "hero-stat-unit", "throughput" }
                                    div { class: "hero-stat-label", "streaming parse" }
                                }
                            }
                            div { class: "hero-demo",
                                div { class: "hero-demo-label",
                                    span { "Simulating 1,000,000 FIX message parse…" }
                                    span { class: "hero-demo-time", "150 ms" }
                                }
                                div { class: "hero-bar-track",
                                    div { class: "hero-bar-fill" }
                                }
                            }
                            p { class: "hero-hint",
                                "Paste FIX data and click "
                                span { class: "hero-hint-kbd", "Process" }
                                "  ·  "
                                span { class: "hero-hint-kbd", "Load file" }
                                "  ·  or pick a sample from the toolbar"
                            }
                        }
                    } else {
                        // ── Input area (above tabs) ──
                        if *loading.read() {
                            div { class: "fix-loading", "Loading file and parsing messages…" }
                        } else if let Some(ref name) = *file_name.read() {
                            {
                                let files = loaded_files.read();
                                let file_count = files.len();
                                let expanded = *show_file_list.read();
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

                        // ── Panel view tabs (Timeline / Lifecycle / Overview / Validate) ──
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
                                    class: if in_lifecycle { "panel-tab panel-tab-active" } else { "panel-tab" },
                                    onclick: move |_| view_mode.set(ViewMode::Lifecycle),
                                    "Trade Latency"
                                }
                                button {
                                    class: if in_overview { "panel-tab panel-tab-active" } else { "panel-tab" },
                                    onclick: move |_| view_mode.set(ViewMode::Overview),
                                    "Session Analysis"
                                }
                                button {
                                    class: if in_validator { "panel-tab panel-tab-active" } else { "panel-tab" },
                                    onclick: move |_| view_mode.set(ViewMode::Validator),
                                    "FIX Validator"
                                }
                            }
                        }

                        // ── Main content ──
                        if in_validator {
                            validator_panel { messages: messages }
                        } else if in_overview {
                            overview_panel {
                                messages: messages,
                                pro: pro,
                            }
                        } else if in_lifecycle {
                            lifecycle_panel {
                                messages: messages,
                                selected_idx: selected_idx,
                                pro: pro,
                            }
                        } else {
                            div { class: "panels",
                                timeline_panel {
                                    messages: messages,
                                    selected_idx: selected_idx,
                                    skip_heartbeats: skip_heartbeats,
                                    parse_stats: parse_stats,
                                    pro: pro,
                                }
                                detail_panel {
                                    detail_msg: detail_msg,
                                    skip_common: skip_common,
                                }
                            }
                        }
                    }
                }

                // ── Resize handle (always visible, centered collapse buttons) ──
                // Drag is handled by the persistent native JS handler set up in use_effect above.
                div {
                    class: "resize-handle",
                    // Visual bar only
                    div { class: "resize-handle-bar" }
                    // Centered collapse/expand buttons
                    div { class: "collapse-panel-btns",
                        button {
                            class: "collapse-panel-btn",
                            title: if right_collapsed { "Expand right panel" } else { "Collapse right panel" },
                            onclick: move |_| {
                                let c = *right_panel_collapsed.read();
                                if c {
                                    // Expanding: signal already holds the saved width
                                    right_panel_collapsed.set(false);
                                } else {
                                    // Collapsing: read actual DOM width first so we survive any
                                    // JS↔Rust async lag, then collapse
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

                // ── Right: premium panel ──
                div {
                    id: "premium-panel-main",
                    class: "premium-panel",
                    style: if right_collapsed {
                        "width: 0; min-width: 0; overflow: hidden; border: none;".to_string()
                    } else if left_collapsed {
                        // Left is gone — right fills everything
                        "flex: 1; min-width: 0;".to_string()
                    } else {
                        format!("flex-shrink: 0; width: {right_w}px; min-width: 0;")
                    },

                    // Panel header
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

                        // Feature cards (scrollable)
                        div { class: "premium-panel-scroll",

                            // Trade Latency Analysis
                            div { class: "feature-card",
                                div { class: "feature-card-top",
                                    span { class: "feature-card-name", "Trade Latency Analysis" }
                                    span { class: "badge badge-gray feature-badge", "Active" }
                                }
                                p { class: "feature-card-desc",
                                    "Reconstruct full order chains from RFQ to fill, with latency at each hop."
                                }
                                if has_messages {
                                    button {
                                        class: "btn-feature",
                                        onclick: move |_| {
                                            if in_lifecycle {
                                                view_mode.set(ViewMode::Timeline);
                                            } else {
                                                view_mode.set(ViewMode::Lifecycle);
                                            }
                                        },
                                        if in_lifecycle { "← Back to Timeline" } else { "View Lifecycle →" }
                                    }
                                } else {
                                    span { class: "feature-card-hint", "Load data to use" }
                                }
                            }

                            // Session Analysis
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
                                            if in_overview {
                                                view_mode.set(ViewMode::Timeline);
                                            } else {
                                                view_mode.set(ViewMode::Overview);
                                            }
                                        },
                                        if in_overview { "← Back to Timeline" } else { "View Report →" }
                                    }
                                } else {
                                    span { class: "feature-card-hint", "Load data to use" }
                                }
                            }

                            // FIX Validator
                            div { class: "feature-card",
                                div { class: "feature-card-top",
                                    span { class: "feature-card-name", "FIX Validator" }
                                    span { class: "badge badge-gray feature-badge", "Active" }
                                }
                                p { class: "feature-card-desc",
                                    "Validate messages against FIX spec, check required tags, enums, checksums & consistency rules."
                                }
                                if has_messages {
                                    button {
                                        class: "btn-feature",
                                        onclick: move |_| {
                                            if in_validator {
                                                view_mode.set(ViewMode::Timeline);
                                            } else {
                                                view_mode.set(ViewMode::Validator);
                                            }
                                        },
                                        if in_validator { "← Back to Timeline" } else { "Open Validator →" }
                                    }
                                } else {
                                    span { class: "feature-card-hint", "Load data to use" }
                                }
                            }

                            // Order Flow Patterns — combined with AI reject diagnostics (coming soon)
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

                            // AI FIX Builder (new, coming soon)
                            div { class: "feature-card feature-card-soon",
                                div { class: "feature-card-top",
                                    span { class: "feature-card-name", "AI FIX Builder" }
                                    span { class: "badge badge-orange feature-badge", "Soon" }
                                }
                                p { class: "feature-card-desc",
                                    "Talk to AI to generate FIX engine client or server code — \
                                    sessions, message handlers, and schemas tailored to your spec."
                                }
                            }
                        }
                    }
                }
        }
    }
}
